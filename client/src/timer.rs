use async_std::task::sleep;
use chess::{color::Color, game::Game, game_status::GameStatus};
use dioxus::prelude::*;
use std::time::Duration;

#[derive(Props, PartialEq)]
pub struct TimerProps<'a> {
    game: &'a UseRef<Game>,
    start_time: Duration,
}

pub fn Timer<'a>(cx: Scope<'a, TimerProps<'a>>) -> Element<'a> {
    let white_time = use_state(cx, || display_time(cx.props.start_time));
    let black_time = use_state(cx, || display_time(cx.props.start_time));

    use_timer_future(cx, white_time, black_time);

    cx.render(rsx! {
        p { "White time: {white_time}" }
        p { "Black time: {black_time}" }
    })
}

fn display_time(time: Duration) -> String {
    let total_secs = time.as_secs();
    let hours = total_secs / 3600;
    let minutes = total_secs % 3600 / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

fn use_timer_future(cx: Scope<TimerProps>, white_time: &UseState<String>, black_time: &UseState<String>) {
    let active_time_state = match cx.props.game.with(|game| game.get_real_player()) {
        Color::White => white_time.to_owned(),
        Color::Black => black_time.to_owned(),
    };
    let active_time_state = active_time_state.to_owned();
    
    use_future(cx, cx.props.game, |game| {
        let active_time_state = active_time_state.to_owned();
        async move {
            if game.with(|game| game.is_timer_active()) {
                loop {
                    let active_time = game.with(|game| game.get_active_time());
                    let sleep_time = active_time.subsec_micros();
                    sleep(Duration::from_micros(sleep_time as u64)).await;
                    active_time_state.set(display_time(active_time));
                    if active_time.is_zero() {
                        game.with_mut(|game| game.trigger_timeout());
                        return;
                    }
                }
            } else {
                if cx.props.game.with(|game| game.status == GameStatus::NotStarted) {
                    white_time.set(display_time(cx.props.start_time));
                    black_time.set(display_time(cx.props.start_time));
                }
                sleep(Duration::from_secs(u64::MAX)).await;
            }
        }
    });
}
