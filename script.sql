DROP TABLE IF EXISTS users;
CREATE TABLE users
(
    id              SERIAL PRIMARY KEY,
    user_limit      INTEGER NOT NULL,
    user_balance    INTEGER NOT NULL DEFAULT 0
);

INSERT INTO users (id, user_limit, user_balance)
VALUES (DEFAULT, 1000 * 100, 0),
       (DEFAULT, 800 * 100, 0),
       (DEFAULT, 10000 * 100, 0),
       (DEFAULT, 100000 * 100, 0),
       (DEFAULT, 5000 * 100, 0);

DROP TABLE IF EXISTS transactions;
CREATE UNLOGGED TABLE transactions
(
    id          SERIAL PRIMARY KEY,
    user_id     SMALLINT    NOT NULL,
    value       INTEGER     NOT NULL,
    type        CHAR(1)     NOT NULL,
    description VARCHAR(10) NOT NULL,
    created_at       TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

ALTER TABLE
    transactions
    SET
    (autovacuum_enabled = false);

CREATE INDEX user_id_transactions ON transactions (user_id);

