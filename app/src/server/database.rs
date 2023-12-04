use std::env;

use once_cell::sync::OnceCell;
use sqlx::{Error, PgPool, Pool, Postgres};

pub static POOL: OnceCell<Pool<Postgres>> = OnceCell::new();

async fn run_migrations() -> Result<(), Error> {
    Ok(sqlx::migrate!().run(POOL.get().unwrap()).await?)
}

async fn init_db_pool() -> Result<(), Error> {
    POOL.set(
        PgPool::new()
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
