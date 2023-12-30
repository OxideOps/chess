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

#[server(ValidateEmail, "/api")]
pub async fn check_email_exists_then_verify(email: String) -> Result<bool, ServerFnError> {
    let email_info = check_if_email_exists::check_email(email.into()).await;

    Ok(email_info.is_reachable)
    // then verify
}
