//! Per-IP rate limiting for the login endpoint, backed by `tower_governor`'s
//! `governor` rate limiter.
//!
//! Leptos server functions (like `Login`) are auto-mounted by `leptos_axum`
//! alongside every other server function and page route inside a single
//! opaque `Router` build step, so there is no way to attach a
//! `tower_governor::GovernorLayer` to just that one Axum route without
//! either rate-limiting the whole app or every route under `/api`. Instead
//! we build the same `governor` rate limiter `tower_governor` itself uses
//! and check it directly inside the `login` handler, keyed by client IP.

use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

use governor::clock::{Clock, DefaultClock};
use governor::state::keyed::DefaultKeyedStateStore;
use governor::{Quota, RateLimiter};
use tower_governor::key_extractor::{KeyExtractor, PeerIpKeyExtractor};

pub type LoginRateLimiter = RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>;

/// Burst of 30 requests, replenishing 1 every 2s, per client IP. Loosened
/// from the original per-client sizing (burst 5, +1/20s) per D6 in
/// docs/superpowers/specs/2026-07-08-28-abuse-protection-design.md: on DOKS
/// every client shares the LB SNAT address, so this bucket is collective
/// across *all* honest users, permanently - the old constants throttled the
/// 6th concurrent legitimate user in any ~20s window. The real quota
/// protection is the DB-backed caps from WP1 (per-email 5, global 50/24h,
/// 10 attempts/code); this governor is only coarse flood damping. WP4
/// (Cloudflare `CF-Connecting-IP` keying) is the point where per-IP
/// constants can be re-tightened to their original, per-client sizing.
const BURST_SIZE: u32 = 30;
const REPLENISH_PERIOD: Duration = Duration::from_secs(2);

/// Build the login rate limiter. Called once at process startup; the
/// constants above are nonzero by construction so this never panics in
/// practice, but it must not be called from request-handling code.
pub fn build_login_rate_limiter() -> Arc<LoginRateLimiter> {
    let quota = Quota::with_period(REPLENISH_PERIOD)
        .expect("REPLENISH_PERIOD is a nonzero duration")
        .allow_burst(NonZeroU32::new(BURST_SIZE).expect("BURST_SIZE is nonzero"));
    Arc::new(RateLimiter::keyed(quota))
}

/// Confirm-code guard, keyed by client IP like `LoginRateLimiter` but a
/// distinct type so both can be provided via Leptos `expect_context`
/// independently. Wraps the same underlying `governor` limiter since a
/// 6-digit code is a 1M-value space that must not be brute-forceable.
pub struct ConfirmRateLimiter(LoginRateLimiter);

/// Burst of 60 attempts, replenishing 1 every 1s, per client IP. Loosened
/// from the original per-client sizing (burst 10, +1/10s) for the same
/// shared-bucket reason as `BURST_SIZE`/`REPLENISH_PERIOD` above (D6): this
/// governor no longer carries the brute-force protection for the 6-digit
/// code space - that comes from the per-code attempts cap
/// (`CONFIRM_MAX_ATTEMPTS_PER_CODE`, 10, in `server.rs`), which is
/// IP-independent. This limiter is just generous flood damping so
/// concurrent legit users sharing the collective bucket don't starve each
/// other; re-tighten once WP4 (Cloudflare `CF-Connecting-IP` keying) lands.
const CONFIRM_BURST_SIZE: u32 = 60;
const CONFIRM_REPLENISH_PERIOD: Duration = Duration::from_secs(1);

/// Build the confirm rate limiter. Called once at process startup, mirrors
/// `build_login_rate_limiter`.
pub fn build_confirm_rate_limiter() -> Arc<ConfirmRateLimiter> {
    let quota = Quota::with_period(CONFIRM_REPLENISH_PERIOD)
        .expect("CONFIRM_REPLENISH_PERIOD is a nonzero duration")
        .allow_burst(NonZeroU32::new(CONFIRM_BURST_SIZE).expect("CONFIRM_BURST_SIZE is nonzero"));
    Arc::new(ConfirmRateLimiter(RateLimiter::keyed(quota)))
}

/// Returns `Ok(())` if the given IP is within its confirm rate limit, or
/// `Err(wait_seconds)` if it should be rejected.
pub fn check_confirm_rate_limit(limiter: &ConfirmRateLimiter, ip: IpAddr) -> Result<(), u64> {
    limiter.0.check_key(&ip).map_err(|negative| {
        negative
            .wait_time_from(DefaultClock::default().now())
            .as_secs()
    })
}

/// Extract the client IP from the socket's peer address only.
///
/// Client-supplied forwarding headers (`X-Forwarded-For`, `X-Real-Ip`,
/// `Forwarded`) are deliberately ignored: they're trivially spoofable by
/// anyone who can reach the app directly, and on this platform we can't
/// distinguish "set by our trusted LB" from "set by the client" (design
/// decision D6 - the Cilium PROXY-protocol flip needed to recover the real
/// client IP was attempted and permanently reverted by DOKS's managed
/// reconciler, so the peer address the app sees is permanently the LB/node's
/// SNAT address, not the client's). In prod this collapses per-IP limiting
/// to one shared bucket for all honest clients, which is strictly better
/// than trusting spoofable headers; WP1's DB-backed send caps carry the real
/// abuse protection. A `cf-connecting-ip` carve-out is planned post-cutover
/// once Cloudflare fronts the app, but is intentionally not added here.
pub fn extract_client_ip(
    headers: &axum::http::HeaderMap,
    peer_addr: Option<std::net::SocketAddr>,
) -> Option<IpAddr> {
    let mut req = axum::http::Request::new(());
    *req.headers_mut() = headers.clone();
    if let Some(addr) = peer_addr {
        req.extensions_mut()
            .insert(axum::extract::ConnectInfo(addr));
    }
    PeerIpKeyExtractor.extract(&req).ok()
}

/// Returns `Ok(())` if the given IP is within its login rate limit, or
/// `Err(wait_seconds)` if it should be rejected.
pub fn check_login_rate_limit(limiter: &LoginRateLimiter, ip: IpAddr) -> Result<(), u64> {
    limiter.check_key(&ip).map_err(|negative| {
        negative
            .wait_time_from(DefaultClock::default().now())
            .as_secs()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, SocketAddr};

    #[test]
    fn allows_up_to_burst_size_then_rejects() {
        let limiter = build_login_rate_limiter();
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        for _ in 0..BURST_SIZE {
            assert!(check_login_rate_limit(&limiter, ip).is_ok());
        }
        assert!(check_login_rate_limit(&limiter, ip).is_err());
    }

    #[test]
    fn rate_limits_are_tracked_independently_per_ip() {
        let limiter = build_login_rate_limiter();
        let ip_a = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let ip_b = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2));

        for _ in 0..BURST_SIZE {
            assert!(check_login_rate_limit(&limiter, ip_a).is_ok());
        }
        assert!(check_login_rate_limit(&limiter, ip_a).is_err());
        // A different IP still has its own untouched quota.
        assert!(check_login_rate_limit(&limiter, ip_b).is_ok());
    }

    #[test]
    fn spoofed_forwarding_headers_do_not_select_the_key() {
        // A client can set any of these to whatever it likes; none of them
        // may override the socket peer address (D6: real client IPs are
        // permanently unavailable behind the LB, so the peer is all we can
        // trust).
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.7".parse().unwrap());
        headers.insert("x-real-ip", "203.0.113.8".parse().unwrap());
        headers.insert("forwarded", "for=203.0.113.9".parse().unwrap());
        let peer = SocketAddr::from((Ipv4Addr::new(10, 0, 0, 5), 12345));

        let ip = extract_client_ip(&headers, Some(peer));
        assert_eq!(ip, Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5))));
    }

    #[test]
    fn extracts_ip_from_peer_addr() {
        let headers = axum::http::HeaderMap::new();
        let peer = SocketAddr::from((Ipv4Addr::new(10, 0, 0, 5), 12345));

        let ip = extract_client_ip(&headers, Some(peer));
        assert_eq!(ip, Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5))));
    }

    #[test]
    fn returns_none_when_nothing_to_extract_from() {
        let headers = axum::http::HeaderMap::new();
        assert_eq!(extract_client_ip(&headers, None), None);
    }

    #[test]
    fn returns_none_when_only_spoofed_headers_present_and_no_peer_addr() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.7".parse().unwrap());
        assert_eq!(extract_client_ip(&headers, None), None);
    }

    #[test]
    fn confirm_limiter_allows_up_to_burst_size_then_rejects() {
        let limiter = build_confirm_rate_limiter();
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        for _ in 0..CONFIRM_BURST_SIZE {
            assert!(check_confirm_rate_limit(&limiter, ip).is_ok());
        }
        assert!(check_confirm_rate_limit(&limiter, ip).is_err());
    }
}
