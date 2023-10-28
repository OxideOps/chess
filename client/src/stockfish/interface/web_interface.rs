use crate::arrows::Arrows;
use crate::shared_states::Eval;
use crate::stockfish::core::{process_output, MOVES};
use async_std::channel::{unbounded, Receiver, Sender};
use chess::game::Game;
use dioxus::prelude::*;
use js_sys::{Function, Object};
use once_cell::sync::Lazy;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};

pub(crate) type Process = Object;

type Channel = (Sender<String>, Receiver<String>);

static CHANNEL: Lazy<Channel> = Lazy::new(unbounded);

fn get_js_method(object: &Process, method: &str) -> Function {
    js_sys::Reflect::get(object, &method.into())
        .unwrap()
        .dyn_ref::<Function>()
        .unwrap()
        .clone()
}

pub(crate) async fn send_command(process: &mut Process, command: &str) {
    get_js_method(process, "postMessage")
        .call1(process, &command.into())
        .expect("Failed to send stockfish output");
}

pub(crate) async fn run_stockfish() -> Result<Object, JsValue> {
    let sf_promise = js_sys::eval("Stockfish()").unwrap();
    let sf_jsvalue = JsFuture::from(js_sys::Promise::from(sf_promise)).await?;
    let sf_object = sf_jsvalue.dyn_into::<Object>()?;
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
    Ok(sf_object)
}

pub(crate) async fn update_analysis_arrows(
    arrows: &UseLock<Arrows>,
    _process: &UseAsyncLock<Option<Process>>,
    eval_hook: &UseSharedState<Eval>,
    game: &UseSharedState<Game>,
) {
    let mut evals = vec![f64::NEG_INFINITY; MOVES];
    while let Ok(output) = CHANNEL.1.recv().await {
        process_output(&output, &mut evals, &arrows, eval_hook, game).await;
    }
}
