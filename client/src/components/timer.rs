use async_std::task::sleep;
use chess::{color::Color, game::Game};
use dioxus::prelude::*;
use std::time::Duration;

#[derive(Props, PartialEq)]
pub struct TimerProps {
    start_time: Duration,
}

pub fn Timer(cx: Scope<TimerProps>) -> Element {
    let white_time = use_state(cx, || display_time(cx.props.start_time));
    let black_time = use_state(cx, || display_time(cx.props.start_time));
    let player = use_shared_state::<Game>(cx)
        .unwrap()
        .read()
        .get_real_player();

    use_timer_future(cx, white_time, black_time);

    cx.render(rsx! {
        p { background_color: if player == Color::White { "rgba(127, 127, 127, 0.35)" } else { "" }, "White time: {white_time}" }
        p { background_color: if player == Color::Black { "rgba(127, 127, 127, 0.35)" } else { "" }, "Black time: {black_time}" }
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
    white_time: &UseState<String>,
    black_time: &UseState<String>,
) {
    use_future(cx, use_shared_state::<Game>(cx).unwrap(), |game| {
        to_owned![white_time, black_time];
        async move {
            if game.read().is_timer_active() {
                let active_time_state = match game.read().get_real_player() {
                    Color::White => white_time,
                    Color::Black => black_time,
                };
                loop {
                    let active_time = game.read().get_active_time();
                    let sleep_time = active_time.subsec_micros();
                    sleep(Duration::from_micros(sleep_time as u64)).await;
                    active_time_state.set(display_time(active_time));
                    if active_time.is_zero() {
                        game.write().trigger_timeout();
                        return;
                    }
                }
            } else {
                white_time.set(display_time(game.read().get_time(Color::White)));
                black_time.set(display_time(game.read().get_time(Color::Black)));
                sleep(Duration::from_secs(u64::MAX)).await;
            }
        }
    });
}
