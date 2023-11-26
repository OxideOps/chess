use std::env;

use sqlx::{postgres::PgPoolOptions, Error};

pub async fn run_migrations() -> Result<(), Error> {
    sqlx::migrate!()
        .run(
            &PgPoolOptions::new()
                .max_connections(5)
                .connect(&env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
                .await?,
        )
        .await?;

    Ok(())
}
