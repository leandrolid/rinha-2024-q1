use std::env;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use sqlx::{Error, FromRow};
use time::OffsetDateTime;

use crate::{Account, Description, Transaction, TransactionType};

pub struct PostgresRepository {
    pool: PgPool,
}

#[derive(FromRow)]
#[allow(unused)]
pub struct TransactionEntity {
    user_id: i16,
    value: i32,
    kind: String,
    description: String,
    created_at: OffsetDateTime
}

impl PostgresRepository {
    pub async fn connect(url: String) -> Self {
        PostgresRepository {
            pool: PgPoolOptions::new()
                .max_connections(5)
                .connect(&url)
                .await
                .unwrap(),
        }
    }
    pub async fn create_transaction(
        &self,
        user_id: i64,
        transaction: &Transaction,
    ) -> Result<Option<TransactionEntity>, Error> {
        sqlx::query_as(
            "
            INSERT INTO transactions (id, user_id, value, type, description, created_at)
            VALUES (DEFAULT, $1, $2, $3, $4, DEFAULT)
            RETURNING user_id, value, type as kind, description, created_at;
        ",
        )
        .bind(user_id)
        .bind(transaction.value)
        .bind(&transaction.kind.0)
        .bind(&transaction.description.0)
        .fetch_optional(&self.pool)
        .await
    }

    pub fn get_account(&self) -> Option<Account> {
        todo!()
    }
}

#[test]
async fn test_repo() {
    let db_url = env::var("DB_URL").unwrap_or(String::from("postgres://admin:123@localhost/rinha"));

    let repo = PostgresRepository::connect(db_url).await;

    let transaction = Transaction {
        value: 100,
        kind: TransactionType::try_from("c").unwrap(),
        description: Description::try_from("teste").unwrap(),
        ..Default::default()
    };

    repo.create_transaction(1, &transaction)
}
