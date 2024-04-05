use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use axum::{Json, Router};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use serde::{Deserialize, Serialize};
use serde_json::json;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tokio::sync::RwLock;

#[derive(Default)]
struct Account {
    balance: i64,
    limit: i64,
    transactions: RingBuffer,
}

#[derive(Serialize, Deserialize)]
struct RingBuffer(VecDeque<Transaction>);

impl Default for RingBuffer {
    fn default() -> Self {
        Self::with_capacity(10)
    }
}

impl RingBuffer {
    fn with_capacity(capacity: usize) -> Self {
        Self(VecDeque::with_capacity(capacity))
    }
    fn push(&mut self, transaction: Transaction) {
        if self.0.len() == self.0.capacity() {
            self.0.pop_back();
            self.0.push_front(transaction);
        } else {
            self.0.push_front(transaction);
        }
    }
}

enum AccountTransactionError {
    UnableToCreate,
}

impl Account {
    pub fn with_limit(limit: i64) -> Self {
        Account {
            limit,
            ..Default::default()
        }
    }

    pub fn create_transaction(
        &mut self,
        transaction: Transaction,
    ) -> Result<(), AccountTransactionError> {
        match transaction.kind {
            TransactionType::Credit => {
                self.balance += transaction.value;
                self.transactions.push(transaction);
                Ok(())
            }
            TransactionType::Debit => {
                if self.balance + self.limit >= transaction.value {
                    self.balance -= transaction.value;
                    self.transactions.push(transaction);
                    Ok(())
                } else {
                    Err(AccountTransactionError::UnableToCreate)
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
enum TransactionType {
    #[serde(rename = "c")]
    Credit,
    #[serde(rename = "d")]
    Debit,
}

#[derive(Serialize, Deserialize)]
struct Transaction {
    #[serde(rename = "valor")]
    value: i64,
    #[serde(rename = "tipo")]
    kind: TransactionType,
    #[serde(rename = "descricao")]
    description: String,
    #[serde(
    rename = "realizada_em",
    with = "time::serde::rfc3339",
    default = "OffsetDateTime::now_utc"
    )]
    created_at: OffsetDateTime,
}

type AppState = Arc<HashMap<u8, RwLock<Account>>>;

#[tokio::main]
async fn main() {
    let accounts = HashMap::<u8, RwLock<Account>>::from_iter([
        (1, RwLock::new(Account::with_limit(100_000))),
        (2, RwLock::new(Account::with_limit(80_000))),
        (3, RwLock::new(Account::with_limit(1_000_000))),
        (4, RwLock::new(Account::with_limit(10_000_000))),
        (5, RwLock::new(Account::with_limit(500_000))),
    ]);
    let app = Router::new()
        .route("/clientes/:id/transacoes", post(create_transaction))
        .route("/clientes/:id/extrato", get(get_account))
        .with_state(Arc::new(accounts));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn create_transaction(
    Path(account_id): Path<u8>,
    State(accounts): State<AppState>,
    Json(transaction): Json<Transaction>,
) -> impl IntoResponse {
    match accounts.get(&account_id) {
        Some(account) => {
            let mut account = account.write().await;
            match account.create_transaction(transaction) {
                Ok(()) => Ok(Json(json!({
                    "saldo": account.balance,
                    "limite": account.limit,
                }))),
                Err(_) => Err(StatusCode::UNPROCESSABLE_ENTITY),
            }
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn get_account(
    Path(account_id): Path<u8>,
    State(accounts): State<AppState>,
) -> impl IntoResponse {
    match accounts.get(&account_id) {
        Some(account) => {
            let account = account.read().await;
            Ok(Json(json!({
                "saldo": {
                "total": account.balance,
                "data_extrato": OffsetDateTime::now_utc().format(&Rfc3339).unwrap(),
                "limite": account.limit,
            },
            "ultimas_transacoes": account.transactions
            })))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}