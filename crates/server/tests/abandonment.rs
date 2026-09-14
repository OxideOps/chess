//! A player who disconnects and doesn't come back: over real sockets, with
//! a short grace period.

mod common;

use std::time::Duration;

use chess_core::{
    Color,
    protocol::{GameOverReason, GameResult, ServerMessage},
};
use common::*;
use server::Config;

const GRACE: Duration = Duration::from_millis(300);

/// A game with both seats taken and both players connected.
async fn seated_game() -> Option<(String, String, Socket, String, Socket, String)> {
    let db = db().await?;
    let base = serve_with(
        db,
        Config {
            abandon_after: Some(GRACE),
            ..Config::default()
        },
    )
    .await;
    let white_session = guest(&base).await;
    let black_session = guest(&base).await;
    let game = create(&base, &white_session, "{}").await;
    assert_eq!(join(&base, &black_session, &game.id).await.status, 200);
    let mut white = connect(&base, &game.id, Some(&white_session)).await;
    let mut black = connect(&base, &game.id, Some(&black_session)).await;
    assert!(matches!(recv(&mut white).await, ServerMessage::Sync { .. }));
    assert!(matches!(
        recv(&mut black).await,
        ServerMessage::Sync { away: None, .. }
    ));
    Some((base, game.id, white, white_session, black, black_session))
}

fn away_of(msg: &ServerMessage) -> Option<Option<Color>> {
    match msg {
        ServerMessage::AwayChanged { away } => Some(away.map(|a| a.side)),
        _ => None,
    }
}

#[tokio::test]
async fn leaving_after_both_moved_loses_by_abandonment() {
    let Some((base, id, mut white, white_session, mut black, _)) = seated_game().await else {
        return;
    };
    send(&mut white, &mv("e2e4")).await;
    next(&mut black, |m| {
        matches!(m, ServerMessage::MovePlayed { .. })
    })
    .await;
    send(&mut black, &mv("e7e5")).await;
    next(&mut white, |m| {
        matches!(m, ServerMessage::MovePlayed { ply: 2, .. })
    })
    .await;

    black.close(None).await.unwrap();
    let started = next(&mut white, |m| away_of(m).is_some()).await;
    match started {
        ServerMessage::AwayChanged { away: Some(a) } => {
            assert_eq!(a.side, Color::Black);
            assert!(a.ms <= GRACE.as_millis() as u64, "{a:?}");
        }
        other => panic!("{other:?}"),
    }
    match next(&mut white, |m| matches!(m, ServerMessage::GameOver { .. })).await {
        ServerMessage::GameOver { end } => {
            assert_eq!(end.result, GameResult::WhiteWins);
            assert_eq!(end.reason, GameOverReason::Abandoned);
        }
        other => panic!("{other:?}"),
    }
    // Written through: the list shows it finished.
    let r = http(&base, "GET", "/api/me/games", Some(&white_session), "").await;
    assert!(
        r.body.contains(&id) && r.body.contains(r#""reason":"abandoned""#),
        "{}",
        r.body
    );
}

#[tokio::test]
async fn leaving_before_both_moved_aborts() {
    let Some((_, _, mut white, _, black, _)) = seated_game().await else {
        return;
    };
    send(&mut white, &mv("e2e4")).await;
    drop(black); // no close frame: a dropped connection counts too
    match next(&mut white, |m| matches!(m, ServerMessage::GameOver { .. })).await {
        ServerMessage::GameOver { end } => {
            assert_eq!(end.result, GameResult::Aborted);
            assert_eq!(end.reason, GameOverReason::Abandoned);
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn coming_back_in_time_keeps_the_game_going() {
    let Some((base, id, mut white, _, black, black_session)) = seated_game().await else {
        return;
    };
    drop(black);
    assert_eq!(
        away_of(&next(&mut white, |m| away_of(m).is_some()).await),
        Some(Some(Color::Black))
    );
    // Back, in a new tab, before the grace runs out.
    let mut again = connect(&base, &id, Some(&black_session)).await;
    assert!(matches!(
        recv(&mut again).await,
        ServerMessage::Sync {
            away: None,
            your_color: Some(Color::Black),
            ..
        }
    ));
    assert_eq!(
        away_of(&next(&mut white, |m| away_of(m).is_some()).await),
        Some(None)
    );
    // Well past the original deadline, the game is still on.
    tokio::time::sleep(GRACE * 2).await;
    send(&mut white, &mv("e2e4")).await;
    assert!(matches!(
        next(&mut again, |m| !matches!(
            m,
            ServerMessage::AwayChanged { .. }
        ))
        .await,
        ServerMessage::MovePlayed { ply: 1, .. }
    ));

    // Two tabs: closing one of them is not leaving.
    let mut second = connect(&base, &id, Some(&black_session)).await;
    assert!(matches!(
        recv(&mut second).await,
        ServerMessage::Sync { .. }
    ));
    second.close(None).await.unwrap();
    tokio::time::sleep(GRACE * 2).await;
    send(&mut again, &mv("e7e5")).await;
    // White's own move comes back to it first; then Black's, and no game over.
    let msg = next(&mut white, |m| {
        matches!(
            m,
            ServerMessage::MovePlayed { ply: 2, .. } | ServerMessage::GameOver { .. }
        )
    })
    .await;
    assert!(
        matches!(msg, ServerMessage::MovePlayed { ply: 2, .. }),
        "{msg:?}"
    );
}
