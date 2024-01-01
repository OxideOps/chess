use dioxus::prelude::*;

#[component]
pub(crate) fn SignUp(cx: Scope) -> Element {
    let username = use_state(cx, String::new);
    let password = use_state(cx, String::new);
    let email = use_state(cx, String::new);

    cx.render(rsx! {
        form {
            class: "max-w-md mx-auto mt-10 bg-white p-8 border border-gray-300 rounded-lg shadow-sm",
            onsubmit: |_| {
                to_owned![email];
                cx.spawn(async move {
                    if !mailchecker::is_valid(&email.to_string()) {
                        log::error!("email is invalid: {email:?}");
                    }
                    else {
                        log::info!("email: {email:?}");
                    }
                })
            },
            div {
                class: "mb-4",
                label {
                    class: "block text-gray-700 text-sm font-bold mb-2",
                    "Username"
                }
                input {
                    class: "shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline",
                    r#type: "text",
                    value: "{username}",
                    oninput: |e| username.set(e.value.clone()),
                    placeholder: "Username"
                }
            }
            div {
                class: "mb-6",
                label {
                    class: "block text-gray-700 text-sm font-bold mb-2",
                    "Password"
                }
                input {
                    class: "shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 mb-3 leading-tight focus:outline-none focus:shadow-outline",
                    r#type: "password",
                    value: "{password}",
                    oninput: |e| password.set(e.value.clone()),
                    placeholder: "******************"
                }
            }
            div {
                class: "mb-6",
                label {
                    class: "block text-gray-700 text-sm font-bold mb-2",
                    "Email"
                }
                input {
                    class: "shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 mb-3 leading-tight focus:outline-none focus:shadow-outline",
                    r#type: "email",
                    value: "{email}",
                    oninput: |e| email.set(e.value.clone()),
                    placeholder: "email@example.com"
                }
            }
            div {
                class: "flex items-center justify-between",
                button {
                    class: "bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline",
                    "Create Account"
                }
            }
        }
    })
}
