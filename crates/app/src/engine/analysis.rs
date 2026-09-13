//! `use_analysis`: keep an engine pointed at whatever position the UI wants
//! analysed and expose its principal variations as a signal.
//!
//! UCI has no request ids, so a new position is never sent while a search is
//! running: we send `stop`, wait for the `bestmove` that ends the old search,
//! and only then start the new one. That guarantees every line in
//! [`Analysis::lines`] belongs to [`Analysis::fen`].

use chess_core::{
    Color,
    engine::{Line, Message, parse_message},
};
use dioxus::prelude::*;
use futures_util::StreamExt as _;

use super::{Engine, EngineEvent};

/// How far the engine searches before stopping on its own. Deep enough to be
/// useful, shallow enough that an idle tab stops burning CPU.
const MAX_DEPTH: u32 = 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineStatus {
    Loading,
    Ready,
    Failed(String),
}

/// What the engine currently thinks. `lines` always refer to `fen`.
#[derive(Debug, Clone, PartialEq)]
pub struct Analysis {
    pub status: EngineStatus,
    pub name: Option<String>,
    /// The position the lines are for; `None` before the first search.
    pub fen: Option<String>,
    pub turn: Color,
    /// Best lines first (by `multipv`), each the deepest seen for its slot.
    pub lines: Vec<Line>,
    pub searching: bool,
}

impl Default for Analysis {
    fn default() -> Self {
        Self {
            status: EngineStatus::Loading,
            name: None,
            fen: None,
            turn: Color::White,
            lines: Vec::new(),
            searching: false,
        }
    }
}

impl Analysis {
    pub fn best(&self) -> Option<&Line> {
        self.lines.first()
    }

    pub fn depth(&self) -> Option<u32> {
        self.best().map(|l| l.depth)
    }

    fn fail(&mut self, message: String) {
        self.status = EngineStatus::Failed(message);
        self.searching = false;
        self.lines.clear();
    }
}

/// Sequences commands to the engine. See the module docs.
struct Driver {
    engine: Engine,
    multipv: u32,
    ready: bool,
    searching: bool,
    /// The position to search once the engine is free, if any.
    pending: Option<String>,
}

impl Driver {
    /// Ask for `target` to be analysed (`None` = stop analysing).
    fn request(&mut self, target: Option<String>, analysis: &mut Analysis) {
        match target {
            Some(fen)
                if analysis.fen.as_deref() == Some(fen.as_str()) && self.pending.is_none() => {}
            Some(fen) if self.ready && !self.searching => self.start(fen, analysis),
            Some(fen) => {
                self.pending = Some(fen);
                if self.searching {
                    self.engine.send("stop");
                }
            }
            None => {
                self.pending = None;
                if self.searching {
                    self.engine.send("stop");
                }
                analysis.fen = None;
                analysis.lines.clear();
                analysis.searching = false;
            }
        }
    }

    fn start(&mut self, fen: String, analysis: &mut Analysis) {
        self.engine.send(&format!("position fen {fen}"));
        self.engine.send(&format!("go depth {MAX_DEPTH}"));
        self.searching = true;
        analysis.turn = if fen.split_whitespace().nth(1) == Some("b") {
            Color::Black
        } else {
            Color::White
        };
        analysis.fen = Some(fen);
        analysis.lines.clear();
        analysis.searching = true;
    }

    fn handle(&mut self, line: &str, analysis: &mut Analysis) {
        match parse_message(line) {
            Message::UciOk => {
                self.engine
                    .send(&format!("setoption name MultiPV value {}", self.multipv));
                self.engine.send("isready");
            }
            Message::ReadyOk if !self.ready => {
                self.ready = true;
                analysis.status = EngineStatus::Ready;
                if let Some(fen) = self.pending.take() {
                    self.start(fen, analysis);
                }
            }
            Message::ReadyOk => {}
            // Lines that arrive after a `stop` belong to a search we no longer show.
            Message::Info(info) if self.searching && analysis.fen.is_some() => {
                let slot = info.multipv;
                match analysis.lines.iter().position(|l| l.multipv == slot) {
                    Some(i) => analysis.lines[i] = info,
                    None => {
                        analysis.lines.push(info);
                        analysis.lines.sort_by_key(|l| l.multipv);
                    }
                }
            }
            Message::Info(_) => {}
            Message::BestMove { .. } => {
                self.searching = false;
                analysis.searching = false;
                if let Some(fen) = self.pending.take() {
                    self.start(fen, analysis);
                }
            }
            Message::Other(text) => {
                if let Some(name) = text.strip_prefix("id name ") {
                    analysis.name = Some(name.to_string());
                }
            }
        }
    }
}

/// Analyse `target` whenever it changes. `None` pauses the engine. The
/// engine starts on first use and is terminated when the component unmounts.
pub fn use_analysis(target: Memo<Option<String>>, multipv: u32) -> Signal<Analysis> {
    let mut analysis = use_signal(Analysis::default);
    let mut driver: Signal<Option<Driver>> = use_signal(|| None);

    use_hook(move || {
        let (tx, mut rx) = futures_channel::mpsc::unbounded::<EngineEvent>();
        match Engine::start(move |event| {
            let _ = tx.unbounded_send(event);
        }) {
            Ok(engine) => {
                engine.send("uci");
                driver.set(Some(Driver {
                    engine,
                    multipv,
                    ready: false,
                    searching: false,
                    pending: None,
                }));
            }
            Err(message) => analysis.write().fail(message),
        }

        spawn(async move {
            while let Some(event) = rx.next().await {
                match event {
                    EngineEvent::Line(line) => {
                        let mut analysis = analysis.write();
                        if let Some(driver) = driver.write().as_mut() {
                            driver.handle(&line, &mut analysis);
                        }
                    }
                    EngineEvent::Error(message) => {
                        analysis.write().fail(message);
                        driver.set(None);
                    }
                }
            }
        });
    });

    use_effect(move || {
        let target = target();
        let mut analysis = analysis.write();
        if let Some(driver) = driver.write().as_mut() {
            driver.request(target, &mut analysis);
        }
    });

    analysis
}
