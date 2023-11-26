use std::{env, sync::Arc};

use once_cell::sync::OnceCell;
use sqlx::{migrate::Migrator, postgres::PgPoolOptions, Error, Pool, Postgres};

static MIGRATOR: Migrator = sqlx::migrate!();
static POOL: OnceCell<Arc<Pool<Postgres>>> = OnceCell::new();

pub async fn run_migrations() -> Result<(), Error> {
    MIGRATOR.run(&**POOL.get().unwrap()).await?;
    Ok(())
}

pub async fn init_db_pool() -> Result<(), Error> {
    POOL.set(Arc::new(
        PgPoolOptions::new()
            .max_connections(5)
            .connect(&env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
            .await
            .unwrap(),
    ))
    .expect("Failed to initialize database pool");
    Ok(())
}
