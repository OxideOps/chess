use dioxus_fullstack::prelude::*;

#[server(CreateAccount, "/api")]
pub async fn create_account(
    username: String,
    password: String,
    email: String,
) -> Result<(), ServerFnError> {
    use crate::server::auth;

    let hashed_password = auth::hash_password(&password).unwrap();

    database::create_account(&username, &hashed_password, &email).await?;

    // create token, get token from db (automatically created), send token to user, get token from user, validate token is correct from db

    Ok(())
}

#[server(VerifyAccount, "/api")]
pub async fn verify_account(username: String, password: String) -> Result<bool, ServerFnError> {
    use crate::server::auth;

    let hashed_password = database::fetch_password(&username).await?;

    Ok(auth::verify_password(&password, &hashed_password)?)
}
