use dioxus_fullstack::prelude::*;
#[server(DoubleServer)]
async fn double_server(number: usize) -> Result<usize, ServerFnError> {
    // Perform some expensive computation or access a database on the server
    let result = number * 2;
    println!("server calculated {result}");
    Ok(result)
}
