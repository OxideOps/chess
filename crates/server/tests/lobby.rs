//! The seek list: posting, accepting, and the rules around both.
//!
//! Needs `TEST_DATABASE_URL`; skips without it.

mod common;

use chess_core::protocol::{LobbyClientMessage, LobbyServerMessage, ServerMessage};
use common::*;

fn blitz(rated: bool) -> LobbyClientMessage {
    LobbyClientMessage::PostSeek {
        initial_ms: 5 * 60 * 1000,
        increment_ms: 0,
        rated,
    }
}

#[tokio::test]
async fn a_seek_is_posted_seen_and_taken() {
    let Some(db) = db().await else { return };
    let base = serve(db).await;
    let alice = signup(&base, &unique("alice")).await;
    let bob = signup(&base, &unique("bob")).await;

    let mut poster = connect_lobby(&base, Some(&alice)).await;
    let mut watcher = connect_lobby(&base, Some(&bob)).await;
    // Both are told the list on arrival, and it is empty.
    assert_eq!(seeks(&mut poster).await, vec![]);
    assert_eq!(seeks(&mut watcher).await, vec![]);

    send_lobby(&mut poster, &blitz(true)).await;
    let posted = next_lobby(&mut poster, |m| {
        matches!(m, LobbyServerMessage::SeekPosted { .. })
    })
    .await;
    let LobbyServerMessage::SeekPosted { id } = posted else {
        unreachable!()
    };

    // The watcher sees it, with the poster's name and rating on it.
    let listed = seeks(&mut watcher).await;
    assert_eq!(listed.len(), 1, "{listed:?}");
    assert_eq!(listed[0].id, id);
    assert!(listed[0].username.is_some(), "{listed:?}");
    assert!(
        listed[0].rating.is_some(),
        "a signed-up poster has a rating"
    );
    assert!(listed[0].rated);
    assert_eq!(listed[0].initial_ms, 5 * 60 * 1000);

    // Taking it starts one game, and tells both sides which colour they got.
    send_lobby(&mut watcher, &LobbyClientMessage::AcceptSeek { id }).await;
    let started = |m: &LobbyServerMessage| matches!(m, LobbyServerMessage::GameStarted { .. });
    let LobbyServerMessage::GameStarted {
        game_id: poster_game,
        your_color: poster_color,
    } = next_lobby(&mut poster, started).await
    else {
        unreachable!()
    };
    let LobbyServerMessage::GameStarted {
        game_id: watcher_game,
        your_color: watcher_color,
    } = next_lobby(&mut watcher, started).await
    else {
        unreachable!()
    };
    assert_eq!(poster_game, watcher_game, "one game, not two");
    assert_ne!(poster_color, watcher_color, "opposite colours");

    // The seek is gone: someone arriving now sees an empty list.
    let mut latecomer = connect_lobby(&base, None).await;
    assert_eq!(seeks(&mut latecomer).await, vec![]);

    // And the game is real, with both seats filled and playable.
    let mut white = connect(
        &base,
        &poster_game,
        Some(if poster_color.is_white() {
            &alice
        } else {
            &bob
        }),
    )
    .await;
    let ServerMessage::Sync { players, .. } = recv_game(&mut white).await else {
        panic!("expected a sync")
    };
    assert!(
        players.white.is_some() && players.black.is_some(),
        "{players:?}"
    );
    send(&mut white, &mv("e2e4")).await;
    assert!(matches!(
        next(&mut white, |m| matches!(
            m,
            ServerMessage::MovePlayed { .. }
        ))
        .await,
        ServerMessage::MovePlayed { ply: 1, .. }
    ));
}

#[tokio::test]
async fn only_one_of_two_simultaneous_accepts_gets_the_game() {
    let Some(db) = db().await else { return };
    let base = serve(db).await;
    let alice = signup(&base, &unique("alice")).await;
    let bob = signup(&base, &unique("bob")).await;
    let carol = signup(&base, &unique("carol")).await;

    let mut poster = connect_lobby(&base, Some(&alice)).await;
    send_lobby(&mut poster, &blitz(false)).await;
    let LobbyServerMessage::SeekPosted { id } = next_lobby(&mut poster, |m| {
        matches!(m, LobbyServerMessage::SeekPosted { .. })
    })
    .await
    else {
        unreachable!()
    };

    let mut first = connect_lobby(&base, Some(&bob)).await;
    let mut second = connect_lobby(&base, Some(&carol)).await;
    // Both see it before either moves.
    assert_eq!(seeks(&mut first).await.len(), 1);
    assert_eq!(seeks(&mut second).await.len(), 1);

    // Sent back to back, without waiting for the first to be handled.
    let accept = LobbyClientMessage::AcceptSeek { id: id.clone() };
    send_lobby(&mut first, &accept).await;
    send_lobby(&mut second, &accept).await;

    // Exactly one of them is now playing; the other is told it has gone.
    let mut games = Vec::new();
    let mut refusals = Vec::new();
    for socket in [&mut first, &mut second] {
        match next_lobby(socket, |m| {
            matches!(
                m,
                LobbyServerMessage::GameStarted { .. } | LobbyServerMessage::Rejected { .. }
            )
        })
        .await
        {
            LobbyServerMessage::GameStarted { game_id, .. } => games.push(game_id),
            LobbyServerMessage::Rejected { message } => refusals.push(message),
            _ => unreachable!(),
        }
    }
    assert_eq!(games.len(), 1, "one winner, got {games:?}");
    assert_eq!(refusals.len(), 1, "one loser, got {refusals:?}");
    assert!(
        refusals[0].contains("no longer open"),
        "the loser should be told why: {}",
        refusals[0]
    );

    // The poster is in that one game, and the seek is gone.
    let LobbyServerMessage::GameStarted { game_id, .. } = next_lobby(&mut poster, |m| {
        matches!(m, LobbyServerMessage::GameStarted { .. })
    })
    .await
    else {
        unreachable!()
    };
    assert_eq!(game_id, games[0]);
    let mut latecomer = connect_lobby(&base, None).await;
    assert_eq!(seeks(&mut latecomer).await, vec![]);
}

#[tokio::test]
async fn rated_seeks_are_for_accounts_only() {
    let Some(db) = db().await else { return };
    let base = serve(db).await;
    let visitor = guest(&base).await;
    let account = signup(&base, &unique("dana")).await;

    // A guest can't post one.
    let mut guest_socket = connect_lobby(&base, Some(&visitor)).await;
    send_lobby(&mut guest_socket, &blitz(true)).await;
    let LobbyServerMessage::Rejected { message } = next_lobby(&mut guest_socket, |m| {
        matches!(m, LobbyServerMessage::Rejected { .. })
    })
    .await
    else {
        unreachable!()
    };
    assert!(message.contains("account"), "{message}");

    // Nor take someone else's.
    let mut owner = connect_lobby(&base, Some(&account)).await;
    send_lobby(&mut owner, &blitz(true)).await;
    let LobbyServerMessage::SeekPosted { id } = next_lobby(&mut owner, |m| {
        matches!(m, LobbyServerMessage::SeekPosted { .. })
    })
    .await
    else {
        unreachable!()
    };
    send_lobby(
        &mut guest_socket,
        &LobbyClientMessage::AcceptSeek { id: id.clone() },
    )
    .await;
    let LobbyServerMessage::Rejected { message } = next_lobby(&mut guest_socket, |m| {
        matches!(m, LobbyServerMessage::Rejected { .. })
    })
    .await
    else {
        unreachable!()
    };
    assert!(message.contains("account"), "{message}");
    // It is still there for someone who can take it.
    assert_eq!(seeks(&mut owner).await.len(), 1);

    // A casual seek is fine for a guest, either way round.
    let mut casual = connect_lobby(&base, Some(&visitor)).await;
    send_lobby(&mut casual, &blitz(false)).await;
    assert!(matches!(
        next_lobby(&mut casual, |m| matches!(
            m,
            LobbyServerMessage::SeekPosted { .. } | LobbyServerMessage::Rejected { .. }
        ))
        .await,
        LobbyServerMessage::SeekPosted { .. }
    ));
}

#[tokio::test]
async fn a_seek_does_not_outlive_its_connection() {
    let Some(db) = db().await else { return };
    let base = serve(db).await;
    let alice = signup(&base, &unique("alice")).await;
    let bob = signup(&base, &unique("bob")).await;

    let mut watcher = connect_lobby(&base, Some(&bob)).await;
    assert_eq!(seeks(&mut watcher).await, vec![]);

    {
        let mut poster = connect_lobby(&base, Some(&alice)).await;
        send_lobby(&mut poster, &blitz(false)).await;
        assert_eq!(seeks(&mut watcher).await.len(), 1);

        // Posting again replaces it rather than stacking up.
        send_lobby(&mut poster, &blitz(false)).await;
        assert_eq!(seeks(&mut watcher).await.len(), 1);

        // Cancelling withdraws it.
        send_lobby(&mut poster, &LobbyClientMessage::CancelSeek).await;
        assert_eq!(seeks(&mut watcher).await, vec![]);

        send_lobby(&mut poster, &blitz(false)).await;
        assert_eq!(seeks(&mut watcher).await.len(), 1);
        // ...and the socket closes here, with a seek still on the list.
    }
    // Closing the tab takes the seek with it: nobody is left waiting on
    // someone who has gone.
    assert_eq!(seeks(&mut watcher).await, vec![]);
}

#[tokio::test]
async fn you_cannot_play_yourself() {
    let Some(db) = db().await else { return };
    let base = serve(db).await;
    let alice = signup(&base, &unique("alice")).await;

    let mut one = connect_lobby(&base, Some(&alice)).await;
    send_lobby(&mut one, &blitz(false)).await;
    let LobbyServerMessage::SeekPosted { id } = next_lobby(&mut one, |m| {
        matches!(m, LobbyServerMessage::SeekPosted { .. })
    })
    .await
    else {
        unreachable!()
    };

    // Same account, another tab.
    let mut other_tab = connect_lobby(&base, Some(&alice)).await;
    send_lobby(&mut other_tab, &LobbyClientMessage::AcceptSeek { id }).await;
    let LobbyServerMessage::Rejected { message } = next_lobby(&mut other_tab, |m| {
        matches!(m, LobbyServerMessage::Rejected { .. })
    })
    .await
    else {
        unreachable!()
    };
    assert!(message.contains("your own"), "{message}");
}
