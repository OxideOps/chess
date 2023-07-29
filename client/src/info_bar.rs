use async_std::task::sleep;
use chess::color::Color;
use chess::game::Game;
use dioxus::prelude::*;
use std::time::Duration;

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

#[derive(Props, PartialEq)]
pub struct InfoBarProps {
    game: UseRef<Game>,
    time: Duration,
    left: u32,
}

fn use_timer_future(
    cx: Scope<InfoBarProps>,
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
                }
            } else {
                sleep(Duration::from_secs(u64::MAX)).await;
            }
        }
    });
}

pub fn InfoBar(cx: Scope<InfoBarProps>) -> Element {
    let white_time = use_state(cx, || display_time(cx.props.time));
    let black_time = use_state(cx, || display_time(cx.props.time));
    let active_time_state = match cx.props.game.with(|game| game.get_current_player()) {
        Color::White => white_time,
        Color::Black => black_time,
    };
    use_timer_future(cx, &cx.props.game, active_time_state);

    cx.render(rsx! {
        div {
            class: "time-container",
            style: "position: absolute; left: {cx.props.left}px; top: 0px",
            p {
                "White time: {white_time}\n",
            },
            p {
                "Black time: {black_time}",
            },
            div {
                class: "moves-container",
                style: "position: relative; overflow-y: auto;",

                "Moves:"
                cx.props.game.with(|game| {
                    game.get_move_history().into_iter().enumerate().map(|(i, turn_str)| {
                        rsx! {
                            p {
                                "{i + 1} {turn_str}"
                            }
                        }
                    })
                })
            }
        },
    })
}
