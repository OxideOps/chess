use crate::arrows::{ArrowData, Arrows, ALPHA};
use crate::stockfish_interface::{run_stockfish, send_command, update_analysis_arrows, Process};
use chess::game::Game;
use chess::moves::Move;
use dioxus::prelude::*;
use regex::Regex;

pub const MOVES: usize = 10;
pub const THREADS: usize = 4;
pub const DEPTH: usize = 40;
pub const HASH: usize = 256;

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
    get_info(output, "score cp")
        .unwrap()
        .parse::<f64>()
        .unwrap()
        / 20.0
}

// Makes it so the arrow for the best move has the default ALPHA value
fn eval_to_alpha(eval: f64, evals: &[f64]) -> f64 {
    sigmoid(inv_sigmoid(ALPHA) + eval - evals.iter().max_by(|a, b| a.total_cmp(b)).unwrap())
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
                update_position(
                    game.with(|game| game.get_fen_str()),
                    stockfish_process.to_owned(),
                )
                .await;
                update_analysis_arrows(&arrows, stockfish_process).await;
            }
            Err(err) => log::error!("Failed to start stockfish: {err:?}"),
        }
    } else {
        stockfish_process.with_mut(|option| {
            if let Some(process) = option {
                stop_stockfish(process);
                *option = None;
                arrows.set(Arrows::new(vec![Move::default(); MOVES]));
            }
        })
    }
}

pub fn process_output(output: &str, evals: &mut [f64], arrows: &UseRef<Arrows>) {
    if let Some(i) = get_info(output, "multipv") {
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
    } else if output == "readyok" {
        arrows.set(Arrows::new(vec![Move::default(); MOVES]));
    }
}

pub fn init_stockfish(process: &mut Process) {
    log::info!("Starting Stockfish");
    send_command(process, "uci");
    send_command(process, &format!("setoption name MultiPV value {MOVES}"));
    send_command(process, &format!("setoption name Threads value {THREADS}"));
    send_command(process, &format!("setoption name Hash value {HASH}"));
}

pub fn stop_stockfish(process: &mut Process) {
    log::info!("Stopping Stockfish");
    send_command(process, "stop");
    send_command(process, "isready");
    send_command(process, "quit");
}

pub async fn update_position(fen_str: String, process: UseRef<Option<Process>>) {
    process.with_mut(|process| {
        if let Some(process) = process {
            log::debug!("Setting stockfish position: {fen_str:?}");
            send_command(process, "stop");
            send_command(process, &format!("position fen {fen_str}"));
            send_command(process, "isready");
            send_command(process, &format!("go depth {DEPTH}"));
        }
    });
}
