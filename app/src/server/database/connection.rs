use std::env;

use sqlx::{migrate::Migrator, postgres::PgPoolOptions, Error};

static MIGRATOR: Migrator = sqlx::migrate!();

pub async fn run_migrations() -> Result<(), Error> {
    MIGRATOR
        .run(
            &PgPoolOptions::new()
                .max_connections(5)
                .connect(&env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
                .await?,
        )
        .await?;

    Ok(())
}
