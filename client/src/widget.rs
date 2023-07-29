use crate::board::Board;
use crate::info_bar::InfoBar;

use chess::game::Game;
use chess::player::Player;
use dioxus::prelude::*;
use std::time::Duration;

const BOARD_SIZE: u32 = 800;

#[derive(Props, PartialEq)]
pub struct WidgetProps {
    white_player: Player,
    black_player: Player,
    time: Duration,
}

pub fn Widget(cx: Scope<WidgetProps>) -> Element {
    let game = use_ref(cx, || Game::builder().duration(cx.props.time).build());
    cx.render(rsx! {
        style { include_str!("../../styles/widget.css") }
        // div for widget
        div {
            autofocus: true,
            tabindex: 0,
            // event handlers
            onmousedown: |event| mouse_down_state.set(Some(event.client_coordinates())),
            onmouseup: move |event| handle_on_mouse_up_event(event, game, cx.props, mouse_down_state, dragging_point_state, game_socket),
            onmousemove: move |event| handle_on_mouse_move_event(event, game, mouse_down_state, dragging_point_state),
            onkeydown: move |event| handle_on_key_down(&event.key(), game),
            //board
            img {
                src: "images/board.png",
                class: "images",
                style: "left: 0; top: 0;",
                width: "{BOARD_SIZE}",
                height: "{BOARD_SIZE}",
            },
            // pieces
            game.with(|game| game.get_pieces()).into_iter().map(|(piece, pos)| {
                let (top_left, z_index) = get_positions(&pos, mouse_down_state, dragging_point_state);
                rsx! {
                    img {
                        src: "{get_piece_image_file(piece)}",
                        class: "images",
                        style: "left: {top_left.x}px; top: {top_left.y}px; z-index: {z_index}",
                        width: "{BOARD_SIZE / 8}",
                        height: "{BOARD_SIZE / 8}",
                    }
                }
            }),
            // info bar
            div {
                class: "time-container",
                style: "position: absolute; left: {BOARD_SIZE}px; top: 0px",
                p {
                    "White time: {white_time}\n",
                },
                p {
                    "Black time: {black_time}",
                },
                div {
                    class: "moves-container",
                    style: "position: relative; overflow-y: auto;",

                    p{ "Rounds:" }
                    game.with(|game| {
                        game.get_move_history().enumerate().map(|(i, mv_str)| {
                            rsx! {
                                tr {
                                    td {
                                        style: "padding-right: 15px;" ,
                                        "{i}." }
                                    td {
                                        style: "padding-right: 15px;",
                                        "{mv_str}" 
                                    }
                                }
                            }
                        })
                    })
                }
            },
        Board {
            size: BOARD_SIZE,
            game: game,
            white_player_kind: cx.props.white_player.kind,
            black_player_kind: cx.props.black_player.kind,
        },
        InfoBar {
            game: game,
            time: cx.props.time,
            left: BOARD_SIZE,
        }
    })
}
