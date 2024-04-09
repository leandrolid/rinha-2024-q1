use axum::http::StatusCode;
use serde::Serialize;
use sqlx::postgres::PgPoolOptions;
use sqlx::FromRow;
use sqlx::PgPool;
use time::OffsetDateTime;

use crate::Transaction;

pub struct PostgresRepository {
    pool: PgPool,
}

#[derive(FromRow)]
pub struct TransactionResult {
    pub user_limit: i32,
    pub user_balance: i32,
}

#[derive(FromRow, Serialize)]
pub struct AccountTransaction {
    #[serde(skip_serializing)]
    pub user_balance: i32,
    #[serde(skip_serializing)]
    pub user_limit: i32,
    #[serde(rename = "valor")]
    pub value: i32,
    #[serde(rename = "tipo")]
    pub kind: String,
    #[serde(rename = "descricao")]
    pub description: String,
    #[serde(rename = "realizada_em", with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(FromRow)]
pub struct BalanceResult {
    pub user_balance: i32,
    pub user_limit: i32,
    pub transactions: Vec<AccountTransaction>,
}

#[derive(FromRow)]
pub struct AccountResult {
    pub id: i32,
    pub user_balance: i32,
    pub user_limit: i32,
}

impl PostgresRepository {
    pub async fn connect(url: String) -> Self {
        PostgresRepository {
            pool: PgPoolOptions::new()
                .max_connections(100)
                .connect(&url)
                .await
                .unwrap(),
        }
    }
    pub async fn create_transaction(
        &self,
        user_id: i64,
        transaction: Transaction,
    ) -> Result<TransactionResult, StatusCode> {
        match self.calculate_user_balance(user_id, &transaction).await {
            Ok(transaction_result) => {
                let _ = sqlx::query(
                    "
                            INSERT INTO transactions (id, user_id, value, type, description, created_at)
                            VALUES (DEFAULT, $1, $2, $3, $4, DEFAULT);
                        ",
                )
                .bind(user_id)
                .bind(transaction.value)
                .bind(&transaction.kind.0)
                .bind(&transaction.description.0)
                .fetch_optional(&self.pool)
                .await;

                Ok(transaction_result)
            },
            Err(err) => Err(err),
        }
    }

    pub async fn get_balance(&self, user_id: i64) -> Result<BalanceResult, StatusCode> {
        let account = self.get_account(user_id).await;

        let transactions: Vec<AccountTransaction> = sqlx::query_as(
            "
            SELECT
                t.value as value,
                t.type as kind,
                t.description as description,
                t.created_at as created_at
            FROM transactions as t
            WHERE t.user_id = $1
            LIMIT 10;
        ",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or(vec![]);

        match account {
            Ok(account) => Ok(BalanceResult {
                user_limit: account.user_limit,
                user_balance: account.user_balance,
                transactions,
            }),
            Err(err) => {
                println!("{err}");
                Err(err)
            }
        }
    }

    pub async fn get_account(&self, user_id: i64) -> Result<AccountResult, StatusCode> {
        let account: Result<Option<AccountResult>, sqlx::Error> = sqlx::query_as(
            "
            SELECT
                u.id as id,
                u.user_balance as user_balance,
                u.user_limit as user_limit
            FROM users as u
            WHERE u.id = $1
            LIMIT 1;
        ",
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await;

        match account {
            Ok(account) => match account {
                Some(account) => Ok(account),
                None => Err(StatusCode::NOT_FOUND),
            },
            Err(err) => {
                println!("{err}");
                Err(StatusCode::UNPROCESSABLE_ENTITY)
            }
        }
    }

    pub async fn calculate_user_balance(
        &self,
        user_id: i64,
        transaction: &Transaction,
    ) -> Result<TransactionResult, StatusCode> {
        match self.get_account(user_id).await {
            Ok(mut account) => {
                if transaction.kind.0 == "c" {
                    account.user_balance += transaction.value as i32;
                    self.update_user_balance(user_id, &account).await
                } else if account.user_balance + account.user_limit >= transaction.value as i32 {
                    account.user_balance -= transaction.value as i32;
                    self.update_user_balance(user_id, &account).await
                } else {
                    Err(StatusCode::UNPROCESSABLE_ENTITY)
                }
            }
            Err(err) => Err(err),
        }
    }
    pub async fn update_user_balance(
        &self,
        user_id: i64,
        account_result: &AccountResult,
    ) -> Result<TransactionResult, StatusCode> {
        let result = sqlx::query_as(
            "
                UPDATE users
                SET user_balance = user_balance + $1
                WHERE id = $2
                RETURNING user_limit, user_balance;
                ",
        )
        .bind(account_result.user_balance)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await;

        match result {
            Ok(result) => match result {
                Some(result) => Ok(result),
                None => Err(StatusCode::NOT_FOUND)
            },
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }
}
