use async_std::channel::{unbounded, Receiver, Sender};
use chess::Game;
use dioxus::prelude::*;
use futures_util::TryFutureExt;
use js_sys::{Function, Object};
use once_cell::sync::Lazy;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};

use crate::{
    arrows::Arrows,
    stockfish::{
        core::{process_output, MOVES},
        Eval,
    },
};

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
    let sf_promise = js_sys::eval("Stockfish()")?;
    let sf_jsvalue = JsFuture::from(js_sys::Promise::from(sf_promise)).await?;
    let sf_object = sf_jsvalue.dyn_into::<Object>()?;
    let callback = Closure::wrap(Box::new(|line: JsValue| {
        if let Some(line) = line.as_string() {
            spawn_local(
                CHANNEL
                    .0
                    .send(line)
                    .unwrap_or_else(|e| log::error!("Failed to send stockfish output: {e}")),
            );
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
    let mut scores = vec![f64::NEG_INFINITY; MOVES];
    while let Ok(output) = CHANNEL.1.recv().await {
        process_output(&output, &mut scores, &arrows, eval_hook, game).await;
    }
}
