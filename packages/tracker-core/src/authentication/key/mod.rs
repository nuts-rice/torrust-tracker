//! Tracker authentication services and structs.
//!
//! This module contains functions to handle tracker keys.
//! Tracker keys are tokens used to authenticate the tracker clients when the tracker runs
//! in `private` or `private_listed` modes.
//!
//! There are services to [`generate_key`]  and [`verify_key_expiration`]  authentication keys.
//!
//! Authentication keys are used only by [`HTTP`](crate::servers::http) trackers. All keys have an expiration time, that means
//! they are only valid during a period of time. After that time the expiring key will no longer be valid.
//!
//! Keys are stored in this struct:
//!
//! ```rust,no_run
//! use bittorrent_tracker_core::authentication::Key;
//! use torrust_tracker_primitives::DurationSinceUnixEpoch;
//!
//! pub struct PeerKey {
//!     /// Random 32-char string. For example: `YZSl4lMZupRuOpSRC3krIKR5BPB14nrJ`
//!     pub key: Key,
//!
//!     /// Timestamp, the key will be no longer valid after this timestamp.
//!     /// If `None` the keys will not expire (permanent key).
//!     pub valid_until: Option<DurationSinceUnixEpoch>,
//! }
//! ```
//!
//! You can generate a new key valid for `9999` seconds and `0` nanoseconds from the current time with the following:
//!
//! ```rust,no_run
//! use bittorrent_tracker_core::authentication;
//! use std::time::Duration;
//!
//! let expiring_key = authentication::key::generate_key(Some(Duration::new(9999, 0)));
//!
//! // And you can later verify it with:
//!
//! assert!(authentication::key::verify_key_expiration(&expiring_key).is_ok());
//! ```
pub mod peer_key;
pub mod repository;

use std::panic::Location;
use std::sync::Arc;
use std::time::Duration;

use thiserror::Error;
use torrust_tracker_clock::clock::Time;
use torrust_tracker_located_error::{DynError, LocatedError};
use torrust_tracker_primitives::DurationSinceUnixEpoch;

use crate::CurrentClock;

pub type PeerKey = peer_key::PeerKey;
pub type Key = peer_key::Key;
pub type ParseKeyError = peer_key::ParseKeyError;

/// HTTP tracker authentication key length.
///
/// For more information see function [`generate_key`](crate::authentication::key::generate_key) to generate the
/// [`PeerKey`](crate::authentication::PeerKey).
pub const AUTH_KEY_LENGTH: usize = 32;

/// It generates a new permanent random key [`PeerKey`].
#[must_use]
pub fn generate_permanent_key() -> PeerKey {
    generate_key(None)
}

/// It generates a new random 32-char authentication [`PeerKey`].
///
/// It can be an expiring or permanent key.
///
/// # Panics
///
/// It would panic if the `lifetime: Duration` + Duration is more than `Duration::MAX`.
///
/// # Arguments
///
/// * `lifetime`: if `None` the key will be permanent.
#[must_use]
pub fn generate_key(lifetime: Option<Duration>) -> PeerKey {
    let random_key = Key::random();

    if let Some(lifetime) = lifetime {
        tracing::debug!("Generated key: {}, valid for: {:?} seconds", random_key, lifetime);

        PeerKey {
            key: random_key,
            valid_until: Some(CurrentClock::now_add(&lifetime).unwrap()),
        }
    } else {
        tracing::debug!("Generated key: {}, permanent", random_key);

        PeerKey {
            key: random_key,
            valid_until: None,
        }
    }
}

/// It verifies an [`PeerKey`]. It checks if the expiration date has passed.
/// Permanent keys without duration (`None`) do not expire.
///
/// # Errors
///
/// Will return:
///
/// - `Error::KeyExpired` if `auth_key.valid_until` is past the `current_time`.
/// - `Error::KeyInvalid` if `auth_key.valid_until` is past the `None`.
pub fn verify_key_expiration(auth_key: &PeerKey) -> Result<(), Error> {
    let current_time: DurationSinceUnixEpoch = CurrentClock::now();

    match auth_key.valid_until {
        Some(valid_until) => {
            if valid_until < current_time {
                Err(Error::KeyExpired {
                    location: Location::caller(),
                })
            } else {
                Ok(())
            }
        }
        None => Ok(()), // Permanent key
    }
}

/// Verification error. Error returned when an [`PeerKey`] cannot be
/// verified with the (`crate::authentication::verify_key`) function.
#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum Error {
    #[error("Key could not be verified: {source}")]
    KeyVerificationError {
        source: LocatedError<'static, dyn std::error::Error + Send + Sync>,
    },
    #[error("Failed to read key: {key}, {location}")]
    UnableToReadKey {
        location: &'static Location<'static>,
        key: Box<Key>,
    },
    #[error("Key has expired, {location}")]
    KeyExpired { location: &'static Location<'static> },
}

impl From<r2d2_sqlite::rusqlite::Error> for Error {
    fn from(e: r2d2_sqlite::rusqlite::Error) -> Self {
        Error::KeyVerificationError {
            source: (Arc::new(e) as DynError).into(),
        }
    }
}

#[cfg(test)]
mod tests {

    mod expiring_auth_key {

        use std::time::Duration;

        use torrust_tracker_clock::clock;
        use torrust_tracker_clock::clock::stopped::Stopped as _;

        use crate::authentication;

        #[test]
        fn should_be_displayed() {
            // Set the time to the current time.
            clock::Stopped::local_set_to_unix_epoch();

            let expiring_key = authentication::key::generate_key(Some(Duration::from_secs(0)));

            assert_eq!(
                expiring_key.to_string(),
                format!("key: `{}`, valid until `1970-01-01 00:00:00 UTC`", expiring_key.key) // cspell:disable-line
            );
        }

        #[test]
        fn should_be_generated_with_a_expiration_time() {
            let expiring_key = authentication::key::generate_key(Some(Duration::new(9999, 0)));

            assert!(authentication::key::verify_key_expiration(&expiring_key).is_ok());
        }

        #[test]
        fn should_be_generate_and_verified() {
            // Set the time to the current time.
            clock::Stopped::local_set_to_system_time_now();

            // Make key that is valid for 19 seconds.
            let expiring_key = authentication::key::generate_key(Some(Duration::from_secs(19)));

            // Mock the time has passed 10 sec.
            clock::Stopped::local_add(&Duration::from_secs(10)).unwrap();

            assert!(authentication::key::verify_key_expiration(&expiring_key).is_ok());

            // Mock the time has passed another 10 sec.
            clock::Stopped::local_add(&Duration::from_secs(10)).unwrap();

            assert!(authentication::key::verify_key_expiration(&expiring_key).is_err());
        }
    }
}
