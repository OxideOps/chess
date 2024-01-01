use dioxus_fullstack::prelude::*;

#[server(CreateAccount, "/api")]
pub async fn create_account(username: String, password: String) -> Result<(), ServerFnError> {
    use crate::server::auth;

    let hashed_password = auth::hash_password(&password).unwrap();

    database::create_account(&username, &hashed_password).await?;

    Ok(())
}

#[server(VerifyAccount, "/api")]
pub async fn verify_account(username: String, password: String) -> Result<bool, ServerFnError> {
    use crate::server::auth;

    let hashed_password = database::fetch_password(&username).await?;

    Ok(auth::verify_password(&password, &hashed_password)?)
}
