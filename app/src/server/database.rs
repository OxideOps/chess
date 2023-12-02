use std::env;

use once_cell::sync::OnceCell;
use sqlx::{migrate::Migrator, pool::PoolOptions, Error, Pool, Postgres};

static MIGRATOR: Migrator = sqlx::migrate!();
static POOL: OnceCell<Pool<Postgres>> = OnceCell::new();

async fn run_migrations() -> Result<(), Error> {
    MIGRATOR.run(&*POOL.get().unwrap()).await?;

    Ok(())
}

async fn init_db_pool() -> Result<(), Error> {
    POOL.set(
        PoolOptions::<Postgres>::new()
            .max_connections(5)
            .connect(&env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
            .await
            .unwrap(),
    )
    .expect("database pool must be empty on initialization");

    Ok(())
}

pub async fn connect() -> Result<(), Error> {
    init_db_pool().await?;
    run_migrations().await?;

    Ok(())
}
