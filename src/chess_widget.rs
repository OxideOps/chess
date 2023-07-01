use crate::game::Game;
use crate::pieces::{Piece, Player, Position};
use dioxus::prelude::*;

fn get_piece_image_file(piece: Piece) -> &'static str {
    match piece {
        Piece::Rook(Player::White) => "images/whiteRook.png",
        Piece::Bishop(Player::White) => "images/whiteBishop.png",
        Piece::Pawn(Player::White) => "images/whitePawn.png",
        Piece::Knight(Player::White) => "images/whiteKnight.png",
        Piece::King(Player::White) => "images/whiteKing.png",
        Piece::Queen(Player::White) => "images/whiteQueen.png",
        Piece::Rook(Player::Black) => "images/blackRook.png",
        Piece::Bishop(Player::Black) => "images/blackBishop.png",
        Piece::Pawn(Player::Black) => "images/blackPawn.png",
        Piece::Knight(Player::Black) => "images/blackKnight.png",
        Piece::King(Player::Black) => "images/blackKing.png",
        Piece::Queen(Player::Black) => "images/blackQueen.png",
    }
}

#[inline_props]
pub fn ChessWidget(cx: Scope, size: u32, game: Game) -> Element {
    render! {
        style { include_str!("../styles/chess_widget.css") }
        img {
            src: "images/board.png",
            class: "images",
            style: "left: 0; top: 0;",
            width: "{size}",
            height: "{size}",
        }
        (0..8).flat_map(|x| (0..8).map(move |y| Position { x, y }))
        .filter_map(|pos| game.get_piece(&pos).map(|piece| (pos, piece)))
        .map(|(pos, piece)| rsx! {
            img {
                src: "{get_piece_image_file(piece)}",
                class: "images",
                style: "left: {size * pos.x as u32 / 8}px; top: {size * (7 - pos.y as u32) / 8}px;",
                width: "{size / 8}",
                height: "{size / 8}",
            }
        })
    }
}
