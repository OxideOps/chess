use crate::arrows::Arrows;
use crate::stockfish_client::{init_stockfish, process_output, MOVES};
use async_std::channel::{unbounded, Receiver, Sender};
use dioxus::prelude::*;
use js_sys::{Function, Object};
use once_cell::sync::Lazy;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};

pub type Process = Object;

type Channel = (Sender<String>, Receiver<String>);

static CHANNEL: Lazy<Channel> = Lazy::new(|| unbounded::<String>());

fn get_js_method(object: &Object, method: &str) -> Function {
    js_sys::Reflect::get(object, &method.into())
        .unwrap()
        .dyn_ref::<Function>()
        .unwrap()
        .clone()
}

pub fn send_command(process: &Object, command: &str) {
    get_js_method(process, "postMessage")
        .call1(process, &command.into())
        .expect("Failed to send stockfish output");
}

pub async fn run_stockfish() -> Result<Object, JsValue> {
    let sf_promise = js_sys::eval("Stockfish()").unwrap();
    let sf_jsvalue = JsFuture::from(js_sys::Promise::from(sf_promise)).await?;
    let mut sf_object = sf_jsvalue.dyn_into::<Object>()?;
    let callback = Closure::wrap(Box::new(|line: JsValue| {
        if let Some(line) = line.as_string() {
            spawn_local(async {
                CHANNEL
                    .0
                    .send(line)
                    .await
                    .expect("Failed to send stockfish output");
            });
        }
    }) as Box<dyn FnMut(JsValue)>);
    get_js_method(&sf_object, "addMessageListener")
        .call1(&sf_object, callback.as_ref().unchecked_ref())?;
    callback.forget();

    init_stockfish(&mut sf_object);

    Ok(sf_object)
}

pub async fn update_analysis_arrows(arrows: &UseRef<Arrows>, _process: UseRef<Option<Process>>) {
    let mut evals = vec![f64::NEG_INFINITY; MOVES];
    while let Ok(output) = CHANNEL.1.recv().await {
        process_output(&output, &mut evals, arrows);
    }
}
