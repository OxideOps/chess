use async_std::task::sleep;
use chess::{color::Color, game::Game};
use dioxus::prelude::*;
use std::time::Duration;

#[derive(Props, PartialEq)]
pub struct TimerProps<'a> {
    game: &'a UseRef<Game>,
    time: Duration,
}

pub fn Timer<'a>(cx: Scope<'a, TimerProps<'a>>) -> Element<'a> {
    let white_time = use_state(cx, || display_time(cx.props.time));
    let black_time = use_state(cx, || display_time(cx.props.time));
    let active_time_state = match cx.props.game.with(|game| game.get_real_player()) {
        Color::White => white_time,
        Color::Black => black_time,
    };
    use_timer_future(cx, cx.props.game, active_time_state);

    cx.render(rsx! {
        p { "White time: {white_time}" }
        p { "Black time: {black_time}" }
        button {
            onclick: |_| {
                cx.props.game.with_mut(|game| *game = Game::builder().duration(cx.props.time).build());
                white_time.set(display_time(cx.props.time));
                black_time.set(display_time(cx.props.time));   
            },
            class: "absolute",
            "New Game"
        }
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

fn use_timer_future(
    cx: Scope<TimerProps>,
    game: &UseRef<Game>,
    active_time_state: &UseState<String>,
) {
    use_future(cx, (game,), |(game,)| {
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
                sleep(Duration::from_secs(u64::MAX)).await;
            }
        }
    });
}
