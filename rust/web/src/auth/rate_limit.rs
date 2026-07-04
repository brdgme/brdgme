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
use tower_governor::key_extractor::{KeyExtractor, SmartIpKeyExtractor};

pub type LoginRateLimiter = RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>;

/// Burst of 5 requests, replenishing 1 every 20s, per client IP. Chosen to
/// let a real user retry a typo'd email a couple of times without coming
/// close to draining the Resend free-tier 100/day cap if hammered.
const BURST_SIZE: u32 = 5;
const REPLENISH_PERIOD: Duration = Duration::from_secs(20);

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

/// Burst of 10 attempts, replenishing 1 every 10s, per client IP. Tight
/// enough that brute-forcing the 1M code space is infeasible, loose enough
/// that a user mistyping their code a few times isn't locked out.
const CONFIRM_BURST_SIZE: u32 = 10;
const CONFIRM_REPLENISH_PERIOD: Duration = Duration::from_secs(10);

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

/// Extract the client IP from request headers (`X-Forwarded-For`, `X-Real-Ip`,
/// `Forwarded`) or the socket's peer address, matching
/// `SmartIpKeyExtractor`'s behaviour.
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
    SmartIpKeyExtractor.extract(&req).ok()
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
    fn extracts_ip_from_x_forwarded_for_header() {
        let mut headers = axum::http::HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.7".parse().unwrap());

        let ip = extract_client_ip(&headers, None);
        assert_eq!(ip, Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7))));
    }

    #[test]
    fn falls_back_to_peer_addr_when_no_headers_present() {
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
    fn confirm_limiter_allows_up_to_burst_size_then_rejects() {
        let limiter = build_confirm_rate_limiter();
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        for _ in 0..CONFIRM_BURST_SIZE {
            assert!(check_confirm_rate_limit(&limiter, ip).is_ok());
        }
        assert!(check_confirm_rate_limit(&limiter, ip).is_err());
    }
}
