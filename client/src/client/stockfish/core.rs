use std::{cmp::max, sync::Arc};

use async_std::{
    channel::{unbounded, Receiver, Sender},
    sync::RwLock,
};
use chess::{Color, Game, Move};
use dioxus::prelude::*;
use once_cell::sync::Lazy;
use regex::Regex;

use super::super::{
    arrows::{ArrowData, Arrows, ALPHA},
    helpers::{inv_sigmoid, sigmoid},
    stockfish::{
        interface::{run_stockfish, send_command, update_analysis_arrows, Process},
        Eval,
    },
    system_info::{get_num_cores, get_total_ram},
};

type Channel = (Sender<()>, Receiver<()>);

pub const MOVES: usize = 5;
pub const DEPTH: usize = 30;

static READY_CHANNEL: Lazy<Channel> = Lazy::new(unbounded::<()>);
static IS_READY: Lazy<Arc<RwLock<bool>>> = Lazy::new(|| Arc::new(RwLock::new(true)));

fn get_info<'a>(output: &'a str, key: &'a str) -> Option<&'a str> {
    let re = Regex::new(&format!(r"{key} (\S+)")).unwrap();
    re.captures(output)
        .and_then(|cap| cap.get(1).map(|m| m.as_str()))
}

fn get_eval(output: &str) -> Eval {
    if let Some(mate_in) = get_info(output, "score mate") {
        Eval::Mate(mate_in.parse::<i32>().unwrap())
    } else if let Some(score) = get_info(output, "score cp") {
        Eval::Centipawns(score.parse::<i32>().unwrap())
    } else {
        panic!("Couldn't get stockfish eval for move!")
    }
}

// Makes it so the arrow for the best move has the default ALPHA value
fn score_to_alpha(score: f64, scores: &[f64]) -> f64 {
    sigmoid(inv_sigmoid(ALPHA) + score - scores.iter().max_by(|a, b| a.total_cmp(b)).unwrap())
}

async fn set_ready(ready: bool) {
    *IS_READY.write().await = ready;
}

async fn wait_until_ready(process: &mut Process) {
    set_ready(false).await;
    send_command(process, "isready").await;
    READY_CHANNEL.1.recv().await.ok();
    set_ready(true).await;
}

async fn stop(process: &mut Process) {
    send_command(process, "stop").await;
}

async fn go(process: &mut Process) {
    send_command(process, &format!("go depth {DEPTH}")).await;
}

pub async fn toggle_stockfish(
    analyze: UseState<bool>,
    stockfish_process: UseAsyncLock<Option<Process>>,
    game: UseSharedState<Game>,
    arrows: UseLock<Arrows>,
    eval_hook: UseSharedState<Eval>,
) {
    if *analyze {
        match run_stockfish().await {
            Ok(mut process) => {
                init_stockfish(&mut process).await;
                arrows.set(Arrows::with_size(MOVES));
                update_position(&game.read().get_fen_str(), &mut process).await;
                go(&mut process).await;
                stockfish_process.set(Some(process)).await;
                update_analysis_arrows(&arrows, &stockfish_process, &eval_hook, &game).await;
            }
            Err(err) => log::error!("Failed to start stockfish: {err:?}"),
        }
    // Don't try to use `if let Some(..)` here. It messes with the lock.
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

pub async fn process_output(
    output: &str,
    scores: &mut [f64],
    arrows: &UseLock<Arrows>,
    eval_hook: &UseSharedState<Eval>,
    game: &UseSharedState<Game>,
) {
    if let Some(move_number) = get_info(output, "multipv") {
        if *IS_READY.read().await
            && !arrows.read().is_empty()
            && !(output.contains("upperbound") || output.contains("lowerbound"))
        {
            let i = move_number.parse::<usize>().unwrap() - 1;
            let move_str = get_info(output, " pv").unwrap();
            let mut eval = get_eval(output);
            let score = eval.to_score();
            if i == 0 {
                // clear out old scores when we get a new set
                scores.fill(f64::NEG_INFINITY);
                if game.read().get_current_player() == Color::Black {
                    eval.change_perspective();
                }
                *eval_hook.write() = eval;
            }
            scores[i] = score;
            arrows.write().set(
                i,
                ArrowData::new(
                    Move::from_lan(move_str).unwrap(),
                    score_to_alpha(score, scores),
                ),
            );
        }
    } else if output == "readyok" {
        READY_CHANNEL.0.send(()).await.ok();
    }
}

pub async fn init_stockfish(process: &mut Process) {
    log::info!("Starting Stockfish");
    let threads = max(1, get_num_cores() / 2);
    #[cfg(feature = "desktop")]
    // Use hash size around 50% of total ram in MB that is a multiple of 2048
    let hash = 2048 * (0.0005 * get_total_ram() as f64 / 2048.0).round() as usize;
    #[cfg(feature = "web")]
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
