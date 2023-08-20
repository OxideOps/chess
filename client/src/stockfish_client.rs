use crate::arrows::{ArrowData, Arrows, ALPHA};
use crate::stockfish_interface::{run_stockfish, send_command, update_analysis_arrows, Process};
use async_std::channel::{unbounded, Receiver, Sender};
use chess::game::Game;
use chess::moves::Move;
use dioxus::prelude::*;
use once_cell::sync::Lazy;
use regex::Regex;

type Channel = (Sender<()>, Receiver<()>);

pub const MOVES: usize = 10;
pub const THREADS: usize = 4;
pub const DEPTH: usize = 40;
pub const HASH: usize = 256;
// How much differences in stockfish evaluation affect the alpha of the arrows
const ALPHA_SENSITIVITY: f64 = 1.0 / 20.0;

static READY_CHANNEL: Lazy<Channel> = Lazy::new(unbounded::<()>);

fn get_info<'a>(output: &'a str, key: &'a str) -> Option<&'a str> {
    let re = Regex::new(&format!(r"{key} (\S+)")).unwrap();
    re.captures(output)
        .and_then(|cap| cap.get(1).map(|m| m.as_str()))
}

fn inv_sigmoid(x: f64) -> f64 {
    (x / (1.0 - x)).ln()
}

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

fn get_eval(output: &str) -> f64 {
    ALPHA_SENSITIVITY
        * get_info(output, "score cp")
            .unwrap()
            .parse::<f64>()
            .unwrap()
}

// Makes it so the arrow for the best move has the default ALPHA value
fn eval_to_alpha(eval: f64, evals: &[f64]) -> f64 {
    sigmoid(inv_sigmoid(ALPHA) + eval - evals.iter().max_by(|a, b| a.total_cmp(b)).unwrap())
}

async fn wait_until_ready(process: &UseRef<Option<Process>>) {
    send_command(process, "isready").await;
    READY_CHANNEL.1.recv().await.ok();
}

pub async fn toggle_stockfish(
    analyze: UseState<bool>,
    stockfish_process: UseRef<Option<Process>>,
    game: UseRef<Game>,
    arrows: UseRef<Arrows>,
) {
    if *analyze.get() {
        match run_stockfish().await {
            Ok(process) => {
                stockfish_process.set(Some(process));
                arrows.set(Arrows::new(vec![Move::default(); MOVES]));
                init_stockfish(&stockfish_process).await;
                update_position(game.with(|game| game.get_fen_str()), &stockfish_process).await;
                update_analysis_arrows(arrows, stockfish_process).await;
            }
            Err(err) => log::error!("Failed to start stockfish: {err:?}"),
        }
    } else {
        stop_stockfish(&stockfish_process).await;
        arrows.set(Arrows::default());
        stockfish_process.set(None);
    }
}

pub async fn on_game_changed(
    fen: String,
    process: UseRef<Option<Process>>,
    arrows: UseRef<Arrows>,
) {
    update_position(fen, &process).await;
    wait_until_ready(&process).await;
    arrows.set(Arrows::new(vec![Move::default(); MOVES]));
}

pub async fn process_output(output: &str, evals: &mut [f64], arrows: &UseRef<Arrows>) {
    if let Some(i) = get_info(output, "multipv") {
        if !arrows.read().is_empty() {
            let i = i.parse::<usize>().unwrap() - 1;
            let move_str = get_info(output, " pv").unwrap();
            let eval = get_eval(output);
            evals[i] = eval;
            arrows.write().set(
                i,
                ArrowData::new(
                    Move::from_lan(move_str).unwrap(),
                    eval_to_alpha(eval, evals),
                ),
            );
        }
    } else if output == "readyok" {
        READY_CHANNEL.0.send(()).await.ok();
    }
}

pub async fn init_stockfish(process: &UseRef<Option<Process>>) {
    log::info!("Starting Stockfish");
    send_command(process, "uci").await;
    send_command(process, &format!("setoption name MultiPV value {MOVES}")).await;
    send_command(process, &format!("setoption name Threads value {THREADS}")).await;
    send_command(process, &format!("setoption name Hash value {HASH}")).await;
}

pub async fn stop_stockfish(process: &UseRef<Option<Process>>) {
    log::info!("Stopping Stockfish");
    send_command(process, "stop").await;
    send_command(process, "quit").await;
}

pub async fn update_position(fen_str: String, process: &UseRef<Option<Process>>) {
    log::debug!("Setting stockfish position: {fen_str:?}");
    send_command(process, "stop").await;
    send_command(process, &format!("position fen {fen_str}")).await;
    send_command(process, &format!("go depth {DEPTH}")).await;
}
