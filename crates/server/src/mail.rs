//! Sending email: plain SMTP, or an offline stand-in.
//!
//! Any SMTP provider works (Resend, Postmark, SES, Fastmail…): the server is
//! given a connection URL with the credentials in it (`--smtp-url`,
//! `smtps://user:pass@smtp.example.com:465`) and a From address
//! (`--mail-from`). `--fake-mail` instead writes every message to the log
//! and, with `--fake-mail-dir`, to a file there, so development and the
//! end-to-end tests can follow the links without a network. With neither the
//! server has no [`Mailer`] and the features that send mail are off.

use std::{
    fmt,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use lettre::{
    Address, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    message::{Mailbox, header::ContentType},
};

/// Where messages go. Cheap to clone.
#[derive(Clone)]
pub struct Mailer {
    inner: Arc<Inner>,
}

struct Inner {
    from: Mailbox,
    transport: Transport,
}

enum Transport {
    Smtp(AsyncSmtpTransport<Tokio1Executor>),
    Fake {
        dir: Option<PathBuf>,
        sent: AtomicU64,
    },
}

/// Why a message wasn't sent (or a mailer couldn't be made).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailError(pub String);

impl fmt::Display for MailError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for MailError {}

impl fmt::Debug for Mailer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never the URL: it holds the SMTP password.
        let kind = match &self.inner.transport {
            Transport::Smtp(_) => "smtp",
            Transport::Fake { .. } => "fake",
        };
        write!(f, "Mailer({kind}, from {})", self.inner.from)
    }
}

/// The From address of the offline stand-in.
const FAKE_FROM: &str = "Chess <chess@localhost>";

impl Mailer {
    /// Send through the SMTP server at `url` (`smtp://`, `smtps://`, with
    /// `?tls=required` for STARTTLS), as `from` ("Chess <noreply@…>").
    /// Nothing connects until the first message.
    pub fn smtp(url: &str, from: &str) -> Result<Mailer, MailError> {
        let from: Mailbox = from
            .parse()
            .map_err(|e| MailError(format!("--mail-from {from:?}: {e}")))?;
        let transport = AsyncSmtpTransport::<Tokio1Executor>::from_url(url)
            // The URL is a secret; say what is wrong without echoing it.
            .map_err(|e| MailError(format!("--smtp-url: {e}")))?
            .build();
        Ok(Mailer {
            inner: Arc::new(Inner {
                from,
                transport: Transport::Smtp(transport),
            }),
        })
    }

    /// The offline stand-in: messages go to the log and, given a directory,
    /// one file each there (`<millis>-<n>-<to>.txt`, written whole).
    pub fn fake(dir: Option<PathBuf>) -> Result<Mailer, MailError> {
        if let Some(dir) = &dir {
            std::fs::create_dir_all(dir)
                .map_err(|e| MailError(format!("--fake-mail-dir {}: {e}", dir.display())))?;
        }
        Ok(Mailer {
            inner: Arc::new(Inner {
                from: FAKE_FROM.parse().expect("a valid mailbox"),
                transport: Transport::Fake {
                    dir,
                    sent: AtomicU64::new(0),
                },
            }),
        })
    }

    /// Whether this is the offline stand-in.
    pub fn is_fake(&self) -> bool {
        matches!(self.inner.transport, Transport::Fake { .. })
    }

    /// Send a plain-text message.
    pub async fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), MailError> {
        let address: Address = to
            .parse()
            .map_err(|e| MailError(format!("bad address: {e}")))?;
        let message = Message::builder()
            .from(self.inner.from.clone())
            .to(Mailbox::new(None, address))
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_string())
            .map_err(|e| MailError(e.to_string()))?;
        match &self.inner.transport {
            Transport::Smtp(smtp) => {
                smtp.send(message)
                    .await
                    .map_err(|e| MailError(e.to_string()))?;
                tracing::info!("mail: sent \"{subject}\"");
            }
            Transport::Fake { dir, sent } => {
                let text = format!("To: {to}\nSubject: {subject}\n\n{body}\n");
                tracing::info!("fake mail:\n{text}");
                if let Some(dir) = dir {
                    let n = sent.fetch_add(1, Ordering::Relaxed);
                    let millis = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_millis())
                        .unwrap_or_default();
                    let name = format!("{millis:013}-{n:05}-{}.txt", file_safe(to));
                    // Write then rename, so a reader never sees half a message.
                    let partial = dir.join(format!(".{name}.partial"));
                    tokio::fs::write(&partial, text)
                        .await
                        .map_err(|e| MailError(e.to_string()))?;
                    tokio::fs::rename(&partial, dir.join(name))
                        .await
                        .map_err(|e| MailError(e.to_string()))?;
                }
            }
        }
        Ok(())
    }
}

/// `to` with anything a file name shouldn't hold replaced.
fn file_safe(to: &str) -> String {
    to.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "@.+-_".contains(c) {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

/// A usable address as typed (trimmed), or `None`. Compared elsewhere
/// without regard to case.
pub fn parse_address(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.len() > 254 {
        return None;
    }
    let address: Address = trimmed.parse().ok()?;
    // A bare host ("me@localhost") is valid mail syntax but never someone's
    // address on the internet.
    address.domain().contains('.').then(|| address.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_are_checked_and_trimmed() {
        assert_eq!(
            parse_address(" Ann@Example.com "),
            Some("Ann@Example.com".into())
        );
        assert_eq!(
            parse_address("a+tag@mail.example.org"),
            Some("a+tag@mail.example.org".into())
        );
        assert_eq!(parse_address("no-at-sign"), None);
        assert_eq!(parse_address("two@@example.com"), None);
        assert_eq!(parse_address("me@localhost"), None);
        assert_eq!(parse_address(""), None);
        assert_eq!(
            parse_address(&format!("{}@example.com", "a".repeat(250))),
            None
        );
    }

    // The pool's reaper is a task, so this needs a runtime (main has one).
    #[tokio::test]
    async fn smtp_urls_are_parsed_without_connecting() {
        let mailer = Mailer::smtp(
            "smtps://user:secret@smtp.example.com:465",
            "Chess <noreply@chess.example>",
        )
        .unwrap();
        assert!(!mailer.is_fake());
        let shown = format!("{mailer:?}");
        assert!(!shown.contains("secret"), "{shown}");
        assert!(Mailer::smtp("smtp://relay.example.com?tls=required", "not an address").is_err());
        assert!(Mailer::smtp("http://nope", "Chess <noreply@chess.example>").is_err());
    }

    #[tokio::test]
    async fn the_fake_writes_whole_messages_to_the_directory() {
        let dir = tempfile::tempdir().unwrap();
        let mailer = Mailer::fake(Some(dir.path().to_path_buf())).unwrap();
        mailer
            .send("ann@example.com", "Hello", "a link: http://x/y")
            .await
            .unwrap();
        let files: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(files.len(), 1, "{files:?}");
        assert!(files[0].ends_with("-ann@example.com.txt"), "{files:?}");
        let text = std::fs::read_to_string(dir.path().join(&files[0])).unwrap();
        assert_eq!(
            text,
            "To: ann@example.com\nSubject: Hello\n\na link: http://x/y\n"
        );
        assert!(mailer.send("not an address", "x", "y").await.is_err());
    }
}
