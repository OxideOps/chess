use std::env;

use once_cell::sync::OnceCell;
use sqlx::{pool::PoolOptions, Error, Pool, Postgres};

pub static POOL: OnceCell<Pool<Postgres>> = OnceCell::new();

async fn run_migrations() -> Result<(), Error> {
    Ok(sqlx::migrate!().run(POOL.get().unwrap()).await?)
}

async fn init_db_pool() -> Result<(), Error> {
    POOL.set(
        PoolOptions::<Postgres>::new()
            .max_connections(5)
            .connect(&env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
            .await?,
    )
    .expect("database pool must be empty on initialization");

    Ok(())
}

pub async fn connect() -> Result<(), Error> {
    init_db_pool().await?;
    run_migrations().await
}

pub async fn create_account(username: &str, password: &str) -> Result<(), Error> {
    sqlx::query!(
        "INSERT INTO accounts (username, password) VALUES ($1, $2)",
        username,
        password,
    )
    .execute(POOL.get().unwrap())
    .await?;

    Ok(())
}

pub async fn fetch_password(username: &str) -> Result<String, Error> {
    let record = sqlx::query!(
        "SELECT password FROM accounts WHERE username = $1",
        username
    )
    .fetch_one(POOL.get().unwrap())
    .await?;

    Ok(record.password)
}
