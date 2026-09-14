//! Same-origin check for the game WebSocket.
//!
//! Browsers send the page's `Origin` on an upgrade and attach cookies
//! regardless of who opened the socket, so without this a page on another
//! site could play someone's game for them (`SameSite=Lax` already blocks
//! that in current browsers; this is the belt to that suspender). Requests
//! without an `Origin` come from non-browser clients and are let through:
//! they never had a cookie they didn't choose to send.

use axum::http::{HeaderMap, header};

/// `true` when `Origin` is absent, names the same host the request was
/// sent to, or is one of `allowed` (exact match, e.g.
/// `https://chess.example` when a reverse proxy rewrites `Host`).
pub fn origin_allowed(headers: &HeaderMap, allowed: &[String]) -> bool {
    let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) else {
        return true;
    };
    if allowed.iter().any(|a| a.eq_ignore_ascii_case(origin)) {
        return true;
    }
    let Some(host) = headers.get(header::HOST).and_then(|v| v.to_str().ok()) else {
        return false;
    };
    origin
        .split_once("://")
        .is_some_and(|(_, authority)| authority.eq_ignore_ascii_case(host))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in pairs {
            h.insert(
                header::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                v.parse().unwrap(),
            );
        }
        h
    }

    #[test]
    fn same_host_or_listed_origins_pass_others_fail() {
        let none = &[][..];
        assert!(origin_allowed(&headers(&[("host", "chess.example")]), none));
        assert!(origin_allowed(
            &headers(&[
                ("host", "chess.example"),
                ("origin", "https://chess.example")
            ]),
            none
        ));
        assert!(origin_allowed(
            &headers(&[
                ("host", "127.0.0.1:4173"),
                ("origin", "http://127.0.0.1:4173")
            ]),
            none
        ));
        assert!(!origin_allowed(
            &headers(&[
                ("host", "chess.example"),
                ("origin", "https://evil.example")
            ]),
            none
        ));
        assert!(!origin_allowed(
            &headers(&[
                ("host", "chess.example"),
                ("origin", "https://chess.example.evil")
            ]),
            none
        ));
        assert!(!origin_allowed(
            &headers(&[("host", "chess.example"), ("origin", "null")]),
            none
        ));
        assert!(!origin_allowed(
            &headers(&[("origin", "https://chess.example")]),
            none
        ));
        let listed = vec!["https://chess.example".to_string()];
        assert!(origin_allowed(
            &headers(&[
                ("host", "10.0.0.5:8080"),
                ("origin", "https://chess.example")
            ]),
            &listed
        ));
        assert!(!origin_allowed(
            &headers(&[
                ("host", "10.0.0.5:8080"),
                ("origin", "https://other.example")
            ]),
            &listed
        ));
    }
}
