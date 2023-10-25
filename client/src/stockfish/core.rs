use crate::arrows::{ArrowData, Arrows, ALPHA};
use crate::shared_states::Analyze;
use crate::stockfish::interface::{run_stockfish, send_command, update_analysis_arrows, Process};
use crate::system_info::{get_num_cores, get_total_ram};

use async_std::channel::{unbounded, Receiver, Sender};
use chess::game::Game;
use chess::moves::Move;
use dioxus::prelude::*;
use once_cell::sync::Lazy;
use regex::Regex;

type Channel = (Sender<()>, Receiver<()>);

pub const MOVES: usize = 5;
pub const DEPTH: usize = 30;
// How much differences in stockfish evaluation affect the alpha of the arrows
const ALPHA_SENSITIVITY: f64 = 1.0 / 30.0;
// How much a mate in 1 is worth in centipawns
const MATE_IN_1_EVAL: f64 = 100000.0;
// How much (in centipawns) getting a mate 1 move sooner is worth
const MATE_MOVE_EVAL: f64 = 50.0;

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
        * if let Some(mate_in) = get_info(output, "mate") {
            let mate_in = mate_in.parse::<f64>().unwrap();
            if mate_in >= 0.0 {
                MATE_IN_1_EVAL - MATE_MOVE_EVAL * (mate_in - 1.0)
            } else {
                -MATE_IN_1_EVAL - MATE_MOVE_EVAL * (mate_in + 1.0)
            }
        } else if let Some(score) = get_info(output, "score cp") {
            score.parse::<f64>().unwrap()
        } else {
            panic!("Couldn't get stockfish eval for move!")
        }
}

// Makes it so the arrow for the best move has the default ALPHA value
fn eval_to_alpha(eval: f64, evals: &[f64]) -> f64 {
    sigmoid(inv_sigmoid(ALPHA) + eval - evals.iter().max_by(|a, b| a.total_cmp(b)).unwrap())
}

async fn wait_until_ready(process: &mut Process) {
    send_command(process, "isready").await;
    READY_CHANNEL.1.recv().await.ok();
}

async fn stop(process: &mut Process) {
    send_command(process, "stop").await;
}

async fn go(process: &mut Process) {
    send_command(process, &format!("go depth {DEPTH}")).await;
}

pub async fn toggle_stockfish(
    analyze: UseSharedState<Analyze>,
    stockfish_process: UseAsyncLock<Option<Process>>,
    game: UseSharedState<Game>,
    arrows: UseLock<Arrows>,
) {
    if **analyze.read() {
        match run_stockfish().await {
            Ok(mut process) => {
                init_stockfish(&mut process).await;
                arrows.set(Arrows::with_size(MOVES));
                update_position(&game.read().get_fen_str(), &mut process).await;
                go(&mut process).await;
                stockfish_process.set(Some(process)).await;
                update_analysis_arrows(&arrows, &stockfish_process).await;
            }
            Err(err) => log::error!("Failed to start stockfish: {err:?}"),
        }
    } else if stockfish_process.read().await.is_some() {
        stop_stockfish(stockfish_process.write().await.as_mut().unwrap()).await;
        arrows.set(Arrows::default());
        stockfish_process.set(None).await;
    }
}

pub async fn on_game_changed(
    fen: String,
    process: UseAsyncLock<Option<Process>>,
    arrows: UseLock<Arrows>,
) {
    if let Some(process) = process.write().await.as_mut() {
        stop(process).await;
        update_position(&fen, process).await;
        wait_until_ready(process).await;
        arrows.set(Arrows::with_size(MOVES));
        go(process).await;
    }
}

pub async fn process_output(output: &str, evals: &mut [f64], arrows: &UseLock<Arrows>) {
    if let Some(move_number) = get_info(output, "multipv") {
        if !arrows.read().is_empty()
            && !(output.contains("upperbound") || output.contains("lowerbound"))
        {
            let i = move_number.parse::<usize>().unwrap() - 1;
            let move_str = get_info(output, " pv").unwrap();
            let eval = get_eval(output);
            if i == 0 {
                // clear out old evals when we get a new set
                evals.fill(f64::NEG_INFINITY);
            }
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

pub async fn init_stockfish(process: &mut Process) {
    log::info!("Starting Stockfish");
    // Using all the cores on the system. Should we subtract 1-2 threads to give the UI some room?
    let threads = get_num_cores();
    #[cfg(not(target_arch = "wasm32"))]
    // Use hash size around 50% of total ram in MB that is a multiple of 2048
    let hash = 2048 * (0.0005 * get_total_ram() as f64 / 2048.0).round() as usize;
    #[cfg(target_arch = "wasm32")]
    let hash = 256;
    send_command(process, "uci").await;
    send_command(process, &format!("setoption name MultiPV value {MOVES}")).await;
    send_command(process, &format!("setoption name Threads value {threads}")).await;
    send_command(process, &format!("setoption name Hash value {hash}")).await;
}

async fn stop_stockfish(process: &mut Process) {
    log::info!("Stopping Stockfish");
    stop(process).await;
    send_command(process, "quit").await;
}

async fn update_position(fen_str: &str, process: &mut Process) {
    log::debug!("Setting stockfish position: {fen_str:?}");
    send_command(process, &format!("position fen {fen_str}")).await;
}
