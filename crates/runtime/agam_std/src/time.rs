//! First-party high-resolution monotonic hardware timers and ISO-8601 calendar utilities.
//!
//! Exposes OS high-resolution timers (`std::time::Instant`) and Gregorian calendar
//! date/time conversions powered by `chrono` per `ADOPTED_DEPENDENCIES.md` and `note.md`.

#![deny(clippy::unwrap_used)]

use std::fmt;
use std::time::{Duration, Instant as StdInstant};

use chrono::{DateTime as ChronoDateTime, TimeZone, Utc};

/// Structured time parsing/formatting error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeError {
    pub message: String,
}

impl TimeError {
    pub fn new(message: impl fmt::Display) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

impl fmt::Display for TimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TimeError: {}", self.message)
    }
}

impl std::error::Error for TimeError {}

/// High-resolution monotonic hardware timer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instant {
    inner: StdInstant,
}

impl Instant {
    /// Capture the current monotonic timestamp.
    pub fn now() -> Self {
        Self {
            inner: StdInstant::now(),
        }
    }

    /// Total duration elapsed since the timer was created.
    pub fn elapsed(&self) -> Duration {
        self.inner.elapsed()
    }

    /// Elapsed time in milliseconds.
    pub fn elapsed_ms(&self) -> u128 {
        self.inner.elapsed().as_millis()
    }

    /// Elapsed time in fractional seconds.
    pub fn elapsed_secs(&self) -> f64 {
        self.inner.elapsed().as_secs_f64()
    }
}

/// Sleep the current thread for the specified duration in milliseconds.
pub fn sleep_ms(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

/// Sleep the current thread for the specified duration in microseconds.
pub fn sleep_micros(us: u64) {
    std::thread::sleep(Duration::from_micros(us));
}

/// UTC calendar date and timestamp with ISO-8601 formatting.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DateTime {
    inner: ChronoDateTime<Utc>,
}

impl DateTime {
    /// Return the current UTC date and time.
    pub fn now_utc() -> Self {
        Self { inner: Utc::now() }
    }

    /// Construct a UTC `DateTime` from a Unix timestamp in seconds.
    pub fn from_timestamp_secs(secs: i64) -> Option<Self> {
        Utc.timestamp_opt(secs, 0)
            .single()
            .map(|dt| Self { inner: dt })
    }

    /// Construct a UTC `DateTime` from a Unix timestamp in milliseconds.
    pub fn from_timestamp_millis(millis: i64) -> Option<Self> {
        Utc.timestamp_millis_opt(millis)
            .single()
            .map(|dt| Self { inner: dt })
    }

    /// Format this date/time as an ISO-8601 (RFC 3339) string.
    pub fn to_iso(&self) -> String {
        self.inner.to_rfc3339()
    }

    /// Parse an ISO-8601 (RFC 3339) date/time string.
    pub fn parse_iso(iso_str: &str) -> Result<Self, TimeError> {
        ChronoDateTime::parse_from_rfc3339(iso_str)
            .map(|dt| Self {
                inner: dt.with_timezone(&Utc),
            })
            .map_err(TimeError::new)
    }

    /// Return Unix epoch timestamp in seconds.
    pub fn timestamp_secs(&self) -> i64 {
        self.inner.timestamp()
    }

    /// Return Unix epoch timestamp in milliseconds.
    pub fn timestamp_millis(&self) -> i64 {
        self.inner.timestamp_millis()
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_iso())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monotonic_instant_elapsed() {
        let t0 = Instant::now();
        sleep_ms(1);
        let elapsed = t0.elapsed_ms();
        assert!(elapsed >= 1, "Elapsed ms must be non-zero after sleep");
        assert!(t0.elapsed_secs() >= 0.001);
    }

    #[test]
    fn test_date_time_iso_roundtrip() {
        let now = DateTime::now_utc();
        let iso = now.to_iso();
        let parsed = DateTime::parse_iso(&iso);
        assert!(parsed.is_ok());
        if let Ok(dt) = parsed {
            assert_eq!(dt.timestamp_secs(), now.timestamp_secs());
        }
    }

    #[test]
    fn test_date_time_timestamp_epoch() {
        let epoch = DateTime::from_timestamp_secs(0);
        assert!(epoch.is_some());
        if let Some(dt) = epoch {
            assert_eq!(dt.timestamp_secs(), 0);
            assert_eq!(dt.to_iso(), "1970-01-01T00:00:00+00:00");
        }
    }

    #[test]
    fn test_invalid_iso_returns_error() {
        let bad = DateTime::parse_iso("not-a-date");
        assert!(bad.is_err());
    }
}
