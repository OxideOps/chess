//! The engine process. On the web it is Stockfish compiled to WASM running in
//! a Web Worker; text goes in with `postMessage` and comes back one line per
//! message. Other platforms don't have an engine yet and say so.

/// Something the engine sent back.
#[derive(Debug, Clone)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))] // only the web build has an engine
pub enum EngineEvent {
    /// One line of UCI output.
    Line(String),
    /// The engine crashed or failed to load. No more events will follow.
    Error(String),
}

#[cfg(target_arch = "wasm32")]
pub use web::Engine;

#[cfg(target_arch = "wasm32")]
mod web {
    use super::EngineEvent;
    use dioxus::prelude::*;
    use wasm_bindgen::{JsCast, JsValue, closure::Closure};
    use web_sys::{ErrorEvent, MessageEvent, Worker};

    // Stockfish.js (GPLv3, see assets/engine/COPYING.txt) is loaded at runtime
    // as a separate worker; it is not linked into this binary. The loader
    // finds its `.wasm` from the URL fragment, so the names must be stable.
    const ENGINE_JS: Asset = asset!(
        "/assets/engine/stockfish-18-lite-single.js",
        AssetOptions::js()
            .with_minify(false)
            .with_hash_suffix(false)
    );
    const ENGINE_WASM: Asset = asset!(
        "/assets/engine/stockfish-18-lite-single.wasm",
        AssetOptions::builder().with_hash_suffix(false)
    );

    /// A running engine. Dropping it terminates the worker.
    pub struct Engine {
        worker: Worker,
        _onmessage: Closure<dyn FnMut(MessageEvent)>,
        _onerror: Closure<dyn FnMut(ErrorEvent)>,
    }

    impl Engine {
        pub fn start(on_event: impl Fn(EngineEvent) + 'static) -> Result<Engine, String> {
            let url = format!("{ENGINE_JS}#{ENGINE_WASM}");
            let worker = Worker::new(&url).map_err(|e| describe(&e))?;

            let on_event = std::rc::Rc::new(on_event);
            let on_line = on_event.clone();
            let onmessage = Closure::wrap(Box::new(move |event: MessageEvent| {
                if let Some(line) = event.data().as_string() {
                    on_line(EngineEvent::Line(line));
                }
            }) as Box<dyn FnMut(MessageEvent)>);
            let onerror = Closure::wrap(Box::new(move |event: ErrorEvent| {
                on_event(EngineEvent::Error(event.message()));
            }) as Box<dyn FnMut(ErrorEvent)>);
            worker.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            worker.set_onerror(Some(onerror.as_ref().unchecked_ref()));

            Ok(Engine {
                worker,
                _onmessage: onmessage,
                _onerror: onerror,
            })
        }

        /// Send one UCI command.
        pub fn send(&self, command: &str) {
            if let Err(e) = self.worker.post_message(&JsValue::from_str(command)) {
                dioxus::logger::tracing::warn!(
                    "engine: could not send {command:?}: {}",
                    describe(&e)
                );
            }
        }
    }

    impl Drop for Engine {
        fn drop(&mut self) {
            self.worker.terminate();
        }
    }

    fn describe(e: &JsValue) -> String {
        e.as_string().unwrap_or_else(|| format!("{e:?}"))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::Engine;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::EngineEvent;

    /// Placeholder until the desktop build spawns a native Stockfish.
    pub struct Engine;

    impl Engine {
        pub fn start(_on_event: impl Fn(EngineEvent) + 'static) -> Result<Engine, String> {
            Err("The engine is only available in the web build for now.".into())
        }

        pub fn send(&self, _command: &str) {}
    }
}
