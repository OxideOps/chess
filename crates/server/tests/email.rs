//! An email address on an account and password resets by email, over real
//! HTTP with the fake mailer writing messages to a directory, which is where
//! the tests read the links from.

mod common;

use std::{path::Path, time::Duration};

use common::*;
use server::{Config, account::Account, db::Db, mail::Mailer};
use tempfile::TempDir;

fn unique(prefix: &str) -> String {
    format!(
        "{prefix}{}",
        &uuid::Uuid::new_v4().simple().to_string()[..8]
    )
}

struct MailServer {
    base: String,
    db: Db,
    mail: TempDir,
}

async fn mail_server() -> Option<MailServer> {
    let db = db().await?;
    let mail = tempfile::tempdir().unwrap();
    let base = serve_with(
        db.clone(),
        Config {
            mail: Some(Mailer::fake(Some(mail.path().to_path_buf())).unwrap()),
            ..Config::default()
        },
    )
    .await;
    Some(MailServer { base, db, mail })
}

/// One message from the fake mailer's directory.
#[derive(Debug)]
struct Mail {
    subject: String,
    body: String,
}

impl Mail {
    /// The token in the message's link.
    fn token(&self) -> String {
        let at = self.body.find("token=").expect("a link") + "token=".len();
        self.body[at..]
            .split_whitespace()
            .next()
            .unwrap()
            .to_string()
    }
}

/// Every message sent to `to` so far, oldest first.
fn inbox(dir: &Path, to: &str) -> Vec<Mail> {
    let suffix = format!("-{}.txt", to.to_lowercase());
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.ends_with(&suffix))
        .collect();
    names.sort();
    names
        .iter()
        .map(|n| {
            let text = std::fs::read_to_string(dir.join(n)).unwrap();
            let (head, body) = text.split_once("\n\n").unwrap();
            let subject = head
                .lines()
                .find_map(|l| l.strip_prefix("Subject: "))
                .unwrap()
                .to_string();
            Mail {
                subject,
                body: body.to_string(),
            }
        })
        .collect()
}

/// Wait (up to 5 s) until `to` has `count` messages; returns the newest.
/// Some mail is sent after the reply, so it may not be there yet.
async fn nth_mail(dir: &Path, to: &str, count: usize) -> Mail {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        let mut mails = inbox(dir, to);
        if mails.len() >= count {
            return mails.swap_remove(count - 1);
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "{to} got {} messages, not {count}: {mails:?}",
            mails.len()
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

/// Long enough for mail sent after a reply to have been written.
async fn settle() {
    tokio::time::sleep(Duration::from_millis(300)).await;
}

async fn signup(base: &str, name: &str, password: &str) -> String {
    let body = format!(r#"{{"username":"{name}","password":"{password}"}}"#);
    let r = http(base, "POST", "/api/auth/signup", None, &body).await;
    assert_eq!(r.status, 201, "{}", r.body);
    r.cookie.unwrap()
}

async fn login(base: &str, name: &str, password: &str) -> Reply {
    let body = format!(r#"{{"username":"{name}","password":"{password}"}}"#);
    http(base, "POST", "/api/auth/login", None, &body).await
}

async fn account(base: &str, session: &str) -> Account {
    let r = http(base, "GET", "/api/me/account", Some(session), "").await;
    assert_eq!(r.status, 200, "{}", r.body);
    serde_json::from_str(&r.body).unwrap()
}

async fn put_email(base: &str, session: &str, email: &str) -> Reply {
    let body = format!(r#"{{"email":"{email}"}}"#);
    http(base, "PUT", "/api/me/email", Some(session), &body).await
}

async fn verify(base: &str, token: &str) -> Reply {
    let body = format!(r#"{{"token":"{token}"}}"#);
    http(base, "POST", "/api/auth/verify-email", None, &body).await
}

async fn forgot(base: &str, email: &str) -> Reply {
    let body = format!(r#"{{"email":"{email}"}}"#);
    http(base, "POST", "/api/auth/forgot-password", None, &body).await
}

async fn reset(base: &str, token: &str, password: &str) -> Reply {
    let body = format!(r#"{{"token":"{token}","password":"{password}"}}"#);
    http(base, "POST", "/api/auth/reset-password", None, &body).await
}

/// A signed-up account with `email` verified; returns its session.
async fn verified_account(s: &MailServer, name: &str, email: &str) -> String {
    let session = signup(&s.base, name, "correct horse").await;
    assert_eq!(put_email(&s.base, &session, email).await.status, 200);
    let count = inbox(s.mail.path(), email).len();
    let link = nth_mail(s.mail.path(), email, count).await;
    assert_eq!(verify(&s.base, &link.token()).await.status, 200);
    session
}

#[tokio::test]
async fn an_address_counts_only_once_its_link_is_followed() {
    let Some(s) = mail_server().await else { return };
    let r = http(&s.base, "GET", "/api/auth/mail", None, "").await;
    assert_eq!(r.body, r#"{"enabled":true}"#);

    let name = unique("ann");
    let email = format!("{name}@example.com");
    let session = signup(&s.base, &name, "correct horse").await;
    let a = account(&s.base, &session).await;
    assert_eq!((a.email, a.pending_email), (None, None));

    assert_eq!(
        put_email(&s.base, &session, "not an address").await.status,
        400
    );
    let r = put_email(&s.base, &session, &format!(" {email} ")).await;
    assert_eq!(r.status, 200, "{}", r.body);
    let a: Account = serde_json::from_str(&r.body).unwrap();
    assert_eq!(a.email, None, "not until it is verified");
    assert_eq!(a.pending_email.as_deref(), Some(email.as_str()));
    let link = nth_mail(s.mail.path(), &email, 1).await;
    assert_eq!(link.subject, "Confirm your email address");
    assert!(link.body.contains(&name), "{}", link.body);
    assert!(
        link.body
            .contains(&format!("http://{}/verify-email?token=", s.base)),
        "{}",
        link.body
    );

    // An unverified address can't get a reset: same reply, and no mail.
    assert_eq!(forgot(&s.base, &email).await.status, 202);
    settle().await;
    assert_eq!(inbox(s.mail.path(), &email).len(), 1, "only the link");

    // Following the link verifies it, once.
    let r = verify(&s.base, &link.token()).await;
    assert_eq!(r.status, 200, "{}", r.body);
    assert_eq!(
        r.body,
        format!(r#"{{"email":"{email}","username":"{name}"}}"#)
    );
    assert_eq!(verify(&s.base, &link.token()).await.status, 400);
    assert_eq!(verify(&s.base, "made-up").await.status, 400);
    let a = account(&s.base, &session).await;
    assert_eq!(a.email.as_deref(), Some(email.as_str()));
    assert_eq!(a.pending_email, None);

    // Now a reset goes to it.
    assert_eq!(forgot(&s.base, &email.to_uppercase()).await.status, 202);
    let mail = nth_mail(s.mail.path(), &email, 2).await;
    assert_eq!(mail.subject, "Reset your password");

    // Removing it: the address is told, and resets stop.
    let r = http(&s.base, "DELETE", "/api/me/email", Some(&session), "").await;
    assert_eq!(r.status, 200, "{}", r.body);
    let a: Account = serde_json::from_str(&r.body).unwrap();
    assert_eq!(a.email, None);
    let notice = nth_mail(s.mail.path(), &email, 3).await;
    assert_eq!(notice.subject, "Your email address was removed");
    assert_eq!(
        reset(&s.base, &mail.token(), "brand new pw").await.status,
        400
    );

    // Guests have no account settings; nobody signed in gets 401.
    let guest = guest(&s.base).await;
    assert_eq!(put_email(&s.base, &guest, &email).await.status, 403);
    let r = http(
        &s.base,
        "PUT",
        "/api/me/email",
        None,
        r#"{"email":"a@b.co"}"#,
    )
    .await;
    assert_eq!(r.status, 401);
}

#[tokio::test]
async fn a_reset_link_works_once_and_signs_every_session_out() {
    let Some(s) = mail_server().await else { return };
    let name = unique("bea");
    let email = format!("{name}@example.com");
    let first = verified_account(&s, &name, &email).await;
    let second = login(&s.base, &name, "correct horse").await.cookie.unwrap();

    assert_eq!(forgot(&s.base, &email).await.status, 202);
    let mail = nth_mail(s.mail.path(), &email, 2).await;
    assert!(
        mail.body
            .contains(&format!("http://{}/reset-password?token=", s.base)),
        "{}",
        mail.body
    );
    let token = mail.token();

    // Too short a password is refused without spending the link.
    assert_eq!(reset(&s.base, &token, "short").await.status, 400);
    let r = reset(&s.base, &token, "a new horse").await;
    assert_eq!(r.status, 204, "{}", r.body);

    // Every session is gone, and only the new password works.
    for session in [&first, &second] {
        let r = http(&s.base, "GET", "/api/me", Some(session), "").await;
        assert_eq!(r.status, 401);
    }
    assert_eq!(login(&s.base, &name, "correct horse").await.status, 401);
    assert_eq!(login(&s.base, &name, "a new horse").await.status, 200);

    // The link was spent.
    let r = reset(&s.base, &token, "a third horse").await;
    assert_eq!(r.status, 400);
    assert!(r.body.contains("expired or was already used"), "{}", r.body);
    assert_eq!(login(&s.base, &name, "a new horse").await.status, 200);

    // Asking twice: only the newest link works.
    assert_eq!(forgot(&s.base, &email).await.status, 202);
    let older = nth_mail(s.mail.path(), &email, 3).await.token();
    assert_eq!(forgot(&s.base, &email).await.status, 202);
    let newer = nth_mail(s.mail.path(), &email, 4).await.token();
    assert_eq!(reset(&s.base, &older, "fourth horse").await.status, 400);
    assert_eq!(reset(&s.base, &newer, "fourth horse").await.status, 204);
}

#[tokio::test]
async fn reset_links_expire() {
    let Some(s) = mail_server().await else { return };
    let name = unique("cy");
    let email = format!("{name}@example.com");
    let session = verified_account(&s, &name, &email).await;
    assert_eq!(forgot(&s.base, &email).await.status, 202);
    let token = nth_mail(s.mail.path(), &email, 2).await.token();

    // An hour and a minute later, as far as the database knows.
    let user_id = serde_json::from_str::<serde_json::Value>(
        &http(&s.base, "GET", "/api/me", Some(&session), "")
            .await
            .body,
    )
    .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let expired: bool = sqlx::query_scalar(
        "UPDATE email_tokens SET expires_at = expires_at - interval '61 minutes'
         WHERE user_id = $1 AND purpose = 'reset'
         RETURNING expires_at < now()",
    )
    .bind(&user_id)
    .fetch_one(s.db.pool())
    .await
    .unwrap();
    assert!(expired);

    assert_eq!(reset(&s.base, &token, "too late now").await.status, 400);
    assert_eq!(login(&s.base, &name, "correct horse").await.status, 200);
    let r = http(&s.base, "GET", "/api/me", Some(&session), "").await;
    assert_eq!(r.status, 200, "an expired link signs nobody out");
}

#[tokio::test]
async fn forgot_says_the_same_whether_or_not_there_is_an_account() {
    let Some(s) = mail_server().await else { return };
    let name = unique("dee");
    let known = format!("{name}@example.com");
    let unknown = format!("{}@example.com", unique("nobody"));
    verified_account(&s, &name, &known).await;

    let a = forgot(&s.base, &known).await;
    let b = forgot(&s.base, &unknown).await;
    let without_date = |r: &Reply| -> String {
        r.head
            .lines()
            .filter(|l| !l.starts_with("date: "))
            .collect::<Vec<_>>()
            .join("\n")
    };
    assert_eq!((a.status, &a.body), (202, &String::new()));
    assert_eq!((a.status, &a.body), (b.status, &b.body));
    assert_eq!(without_date(&a), without_date(&b));

    // Only the known address gets anything.
    nth_mail(s.mail.path(), &known, 2).await;
    settle().await;
    assert!(inbox(s.mail.path(), &unknown).is_empty());

    // Not an address at all is a plain 400, account or not.
    assert_eq!(forgot(&s.base, "nope").await.status, 400);
}

#[tokio::test]
async fn resets_are_rate_limited_per_address_and_per_client() {
    let Some(s) = mail_server().await else { return };
    let name = unique("eve");
    let email = format!("{name}@example.com");
    verified_account(&s, &name, &email).await;
    let before = inbox(s.mail.path(), &email).len();

    // Five messages an hour to one address (the verification link was one),
    // the same for an address with no account.
    let unknown = format!("{}@example.com", unique("nobody"));
    for _ in 0..4 {
        assert_eq!(forgot(&s.base, &email).await.status, 202);
    }
    for _ in 0..5 {
        assert_eq!(forgot(&s.base, &unknown).await.status, 202);
    }
    let r = forgot(&s.base, &email).await;
    assert_eq!(r.status, 429, "{}", r.body);
    assert!(r.header("retry-after").is_some());
    assert_eq!(forgot(&s.base, &unknown).await.status, 429);
    nth_mail(s.mail.path(), &email, before + 4).await;
    settle().await;
    assert_eq!(inbox(s.mail.path(), &email).len(), before + 4);

    // Twenty an hour from one client, whatever the addresses: eleven so far
    // (refused ones count too).
    for i in 0..9 {
        let other = format!("{}-{i}@example.com", unique("x"));
        assert_eq!(forgot(&s.base, &other).await.status, 202);
    }
    let other = format!("{}@example.com", unique("x"));
    assert_eq!(forgot(&s.base, &other).await.status, 429);
}

#[tokio::test]
async fn changing_the_address_tells_the_old_one_and_kills_its_links() {
    let Some(s) = mail_server().await else { return };
    let name = unique("fay");
    let old = format!("{name}@example.com");
    let new = format!("{name}@example.org");
    let session = verified_account(&s, &name, &old).await;
    assert_eq!(forgot(&s.base, &old).await.status, 202);
    let old_reset = nth_mail(s.mail.path(), &old, 2).await.token();

    // The change waits for the new address's link; the old one still counts.
    let r = put_email(&s.base, &session, &new).await;
    let a: Account = serde_json::from_str(&r.body).unwrap();
    assert_eq!(a.email.as_deref(), Some(old.as_str()));
    assert_eq!(a.pending_email.as_deref(), Some(new.as_str()));
    let link = nth_mail(s.mail.path(), &new, 1).await;
    assert_eq!(verify(&s.base, &link.token()).await.status, 200);
    assert_eq!(
        account(&s.base, &session).await.email.as_deref(),
        Some(new.as_str())
    );

    let notice = nth_mail(s.mail.path(), &old, 3).await;
    assert_eq!(notice.subject, "Your email address was changed");
    assert!(notice.body.contains(&new), "{}", notice.body);
    // A reset link sent to the old address no longer works, and the old
    // address gets no more.
    assert_eq!(reset(&s.base, &old_reset, "new horse!").await.status, 400);
    assert_eq!(forgot(&s.base, &old).await.status, 202);
    settle().await;
    assert_eq!(inbox(s.mail.path(), &old).len(), 3);

    // Changing back to the verified address drops the pending change.
    put_email(&s.base, &session, &format!("{name}@example.net")).await;
    let r = put_email(&s.base, &session, &new.to_uppercase()).await;
    let a: Account = serde_json::from_str(&r.body).unwrap();
    assert_eq!(
        (a.email.as_deref(), a.pending_email),
        (Some(new.as_str()), None)
    );
}

#[tokio::test]
async fn an_address_verified_elsewhere_is_refused_at_its_link() {
    let Some(s) = mail_server().await else { return };
    let email = format!("{}@example.com", unique("gus"));
    verified_account(&s, &unique("gus"), &email).await;

    // Adding it is accepted like any other (nothing is revealed to the
    // adder); following the link, which only the owner can, says why not.
    let other = signup(&s.base, &unique("hal"), "correct horse").await;
    assert_eq!(put_email(&s.base, &other, &email).await.status, 200);
    let link = nth_mail(s.mail.path(), &email, 2).await;
    let r = verify(&s.base, &link.token()).await;
    assert_eq!(r.status, 409);
    assert!(r.body.contains("another account"), "{}", r.body);
    assert_eq!(account(&s.base, &other).await.email, None);
}

#[tokio::test]
async fn an_address_given_at_signup_gets_its_link() {
    let Some(s) = mail_server().await else { return };
    let name = unique("ida");
    let email = format!("{name}@example.com");
    let body = format!(r#"{{"username":"{name}","password":"correct horse","email":"{email}"}}"#);
    let r = http(&s.base, "POST", "/api/auth/signup", None, &body).await;
    assert_eq!(r.status, 201, "{}", r.body);
    let session = r.cookie.unwrap();
    assert_eq!(
        account(&s.base, &session).await.pending_email.as_deref(),
        Some(email.as_str())
    );
    let link = nth_mail(s.mail.path(), &email, 1).await;
    assert_eq!(verify(&s.base, &link.token()).await.status, 200);

    // A bad address fails the signup before anything is created.
    let body = format!(
        r#"{{"username":"{}","password":"correct horse","email":"x"}}"#,
        unique("joe")
    );
    let r = http(&s.base, "POST", "/api/auth/signup", None, &body).await;
    assert_eq!(r.status, 400, "{}", r.body);
}

#[tokio::test]
async fn without_mail_there_are_no_addresses() {
    let Some(db) = db().await else { return };
    let base = serve(db).await;
    let r = http(&base, "GET", "/api/auth/mail", None, "").await;
    assert_eq!(r.body, r#"{"enabled":false}"#);
    let session = signup(&base, &unique("kit"), "correct horse").await;
    assert_eq!(
        put_email(&base, &session, "kit@example.com").await.status,
        503
    );
    assert_eq!(forgot(&base, "kit@example.com").await.status, 503);
    // An address at signup is ignored rather than failing it.
    let body = format!(
        r#"{{"username":"{}","password":"correct horse","email":"kit@example.com"}}"#,
        unique("kit")
    );
    let r = http(&base, "POST", "/api/auth/signup", None, &body).await;
    assert_eq!(r.status, 201, "{}", r.body);
    let a = account(&base, &r.cookie.unwrap()).await;
    assert_eq!((a.email, a.pending_email), (None, None));
}
