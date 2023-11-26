use sea_orm::{Database, DbErr};

pub async fn connect() -> Result<(), DbErr> {
    Database::connect(std::env::var("DATABASE_URL").expect("DATABASE_URL not set")).await?;
    Ok(())
}
