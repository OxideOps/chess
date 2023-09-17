use dioxus_fullstack::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct UserData {
    pub username: String,
    pub email: String,
}

// #[server(GetUserData, "/api")]
// pub async fn get_user_data() -> Result<UserData, ServerFnError> {
//     let mut client = GLOBAL_CLIENT.lock().await;

//     // Query the database
//     let row = client
//         .query_one("SELECT username, email FROM users WHERE id = 0", &[])
//         .await?;

//     // Extract the data
//     let username: String = row.get("username");
//     let email: String = row.get("email");

//     Ok(UserData { username, email })
// }
