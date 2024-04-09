use std::env;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::repository::PostgresRepository;

mod repository;

#[derive(Serialize, Deserialize, Debug)]
#[serde(try_from = "String")]
struct TransactionType(String);

impl TryFrom<String> for TransactionType {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value == "c" || value == "d" {
            Ok(Self(value))
        } else {
            Err("invalid transaction type")
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(try_from = "String")]
struct Description(String);

impl TryFrom<String> for Description {
    type Error = &'static str;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.is_empty() || value.len() > 10 {
            Err("invalid description")
        } else {
            Ok(Self(value))
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Transaction {
    #[serde(rename = "valor")]
    value: i64,
    #[serde(rename = "tipo")]
    kind: TransactionType,
    #[serde(rename = "descricao")]
    description: Description,
    #[serde(
        rename = "realizada_em",
        with = "time::serde::rfc3339",
        default = "OffsetDateTime::now_utc"
    )]
    created_at: OffsetDateTime,
}

type AppState = Arc<PostgresRepository>;

#[tokio::main]
async fn main() {
    let db_url = env::var("DB_URL").unwrap_or(String::from("postgres://admin:123@localhost/rinha"));

    let repo = PostgresRepository::connect(db_url).await;

    let app = Router::new()
        .route("/clientes/:id/transacoes", post(create_transaction))
        .route("/clientes/:id/extrato", get(get_account))
        .with_state(Arc::new(repo));

    let port = env::var("PORT")
        .ok()
        .and_then(|t| t.parse::<u16>().ok())
        .unwrap_or(9998);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn create_transaction(
    Path(account_id): Path<u8>,
    State(repo): State<AppState>,
    Json(transaction): Json<Transaction>,
) -> impl IntoResponse {
    match repo
        .create_transaction(account_id.into(), transaction)
        .await
    {
        Ok(account) => Ok(Json(json!({
            "saldo": account.user_balance,
            "limite": account.user_limit,
        }))),
        Err(err) => Err(err),
    }
}

async fn get_account(
    Path(account_id): Path<u8>,
    State(repo): State<AppState>,
) -> impl IntoResponse {
    match repo.get_balance(account_id.into()).await {
        Ok(account) => Ok(Json(json!({
            "saldo": {
            "total": account.user_balance,
            "data_extrato": OffsetDateTime::now_utc().format(&Rfc3339).unwrap(),
            "limite": account.user_limit,
        },
        "ultimas_transacoes": account.transactions
        }))),
        Err(err) => Err(err),
    }
}
