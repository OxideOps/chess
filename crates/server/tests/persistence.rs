//! Games survive a "restart": a fresh server on the same database serves the
//! game where the old one left off, seats included.

mod common;

use std::time::Duration;

use chess_core::{
    Color,
    protocol::{ClientMessage, GameOverReason, GameResult, ServerMessage},
};
use common::*;

#[tokio::test]
async fn a_restarted_server_serves_the_same_game() {
    let Some(db) = db().await else { return };

    // First server: two guests, two moves over the socket.
    let base = serve(db.clone()).await;
    let ws = guest(&base).await;
    let bs = guest(&base).await;
    let game = create(&base, &ws, r#"{"initial_ms": 60000, "increment_ms": 0}"#).await;
    join(&base, &bs, &game.id).await;
    let mut white = connect(&base, &game.id, Some(&ws)).await;
    let mut black = connect(&base, &game.id, Some(&bs)).await;
    recv_game(&mut white).await;
    recv_game(&mut black).await;
    send(&mut white, &mv("e2e4")).await;
    recv_game(&mut white).await;
    recv_game(&mut black).await;
    send(&mut black, &mv("e7e5")).await;
    recv_game(&mut white).await;
    recv_game(&mut black).await;
    drop((white, black));

    // The row has the seats and the moves.
    let stored = db.load(&game.id).await.unwrap().unwrap();
    assert!(stored.seats[0].is_some() && stored.seats[1].is_some());
    assert_eq!(stored.snapshot.moves, ["e2e4", "e7e5"]);
    assert!(stored.snapshot.clock_running_for_ms.is_some());
    assert!(db.load("no-such-game").await.unwrap().is_none());

    // "Restart": a fresh server with nothing in memory, same database. The
    // same session cookie still identifies White.
    let base2 = serve(db.clone()).await;
    let mut white = connect(&base2, &game.id, Some(&ws)).await;
    match recv_game(&mut white).await {
        ServerMessage::Sync {
            moves,
            your_color,
            clocks,
            players,
            ..
        } => {
            assert_eq!(
                moves.iter().map(ToString::to_string).collect::<Vec<_>>(),
                ["e2e4", "e7e5"]
            );
            assert_eq!(your_color, Some(Color::White));
            assert!(
                clocks.white_ms <= 60_000 && clocks.white_ms > 50_000,
                "{clocks:?}"
            );
            assert!(players.black.is_some());
        }
        other => panic!("expected Sync, got {other:?}"),
    }
    // And it keeps going: resign, and the end is persisted.
    send(&mut white, &ClientMessage::Resign).await;
    assert!(matches!(
        recv_game(&mut white).await,
        ServerMessage::GameOver { end } if end.result == GameResult::BlackWins && end.reason == GameOverReason::Resignation
    ));
    let mut stored = None;
    for _ in 0..50 {
        stored = db.load(&game.id).await.unwrap();
        if stored.as_ref().is_some_and(|s| s.snapshot.ended.is_some()) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let stored = stored.unwrap();
    assert_eq!(
        stored.snapshot.ended.unwrap().reason,
        GameOverReason::Resignation
    );
    assert_eq!(stored.snapshot.clock_running_for_ms, None);

    // A third server serves the finished game to a spectator.
    let base3 = serve(db.clone()).await;
    let mut watcher = connect(&base3, &game.id, None).await;
    assert!(matches!(
        recv_game(&mut watcher).await,
        ServerMessage::Sync {
            ended: Some(_),
            your_color: None,
            ..
        }
    ));
}

/// Every way a game can end must fit in its row. The matches make adding a
/// result or reason without revisiting the schema a compile error here.
#[tokio::test]
async fn every_result_and_reason_can_be_stored() {
    use GameOverReason::*;
    use GameResult::*;
    use chess_core::protocol::GameEnd;
    use server::room::{Snapshot, TimeControl};

    let Some(db) = db().await else { return };
    let results = [WhiteWins, BlackWins, Draw, Aborted];
    for r in &results {
        match r {
            WhiteWins | BlackWins | Draw | Aborted => {}
        }
    }
    let reasons = [
        Checkmate,
        Resignation,
        Timeout,
        Stalemate,
        InsufficientMaterial,
        Agreement,
        Repetition,
        FiftyMoves,
        Abandoned,
    ];
    for r in &reasons {
        match r {
            Checkmate | Resignation | Timeout | Stalemate | InsufficientMaterial | Agreement
            | Repetition | FiftyMoves | Abandoned => {}
        }
    }
    let ends = results
        .iter()
        .map(|&result| (result, Abandoned))
        .chain(reasons.iter().map(|&reason| (Draw, reason)));
    for (result, reason) in ends {
        let id = uuid::Uuid::new_v4().to_string();
        db.insert(&id, &[None, None], TimeControl::default(), false)
            .await
            .unwrap();
        let end = GameEnd { result, reason };
        let snapshot = Snapshot {
            initial_ms: 300_000,
            increment_ms: 0,
            moves: vec![],
            white_ms: 300_000,
            black_ms: 300_000,
            clock_running_for_ms: None,
            draw_offer: None,
            ended: Some(end),
        };
        db.save_snapshot(&id, &snapshot)
            .await
            .unwrap_or_else(|e| panic!("{end:?}: {e}"));
        let stored = db.load(&id).await.unwrap().unwrap();
        assert_eq!(stored.snapshot.ended, Some(end));
    }
}
