use std::str::FromStr;
use std::time::Duration;

use derive_more::Display;
use rand::distr::Alphanumeric;
use rand::{rng, Rng};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use torrust_tracker_clock::conv::convert_from_timestamp_to_datetime_utc;
use torrust_tracker_primitives::DurationSinceUnixEpoch;

use super::AUTH_KEY_LENGTH;

/// An authentication key which can potentially have an expiration time.
/// After that time is will automatically become invalid.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeerKey {
    /// Random 32-char string. For example: `YZSl4lMZupRuOpSRC3krIKR5BPB14nrJ`
    pub key: Key,

    /// Timestamp, the key will be no longer valid after this timestamp.
    /// If `None` the keys will not expire (permanent key).
    pub valid_until: Option<DurationSinceUnixEpoch>,
}

impl PartialEq for PeerKey {
    fn eq(&self, other: &Self) -> bool {
        // We ignore the fractions of seconds when comparing the timestamps
        // because we only store the seconds in the database.
        self.key == other.key
            && match (&self.valid_until, &other.valid_until) {
                (Some(a), Some(b)) => a.as_secs() == b.as_secs(),
                (None, None) => true,
                _ => false,
            }
    }
}

impl Eq for PeerKey {}

impl std::fmt::Display for PeerKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.expiry_time() {
            Some(expire_time) => write!(f, "key: `{}`, valid until `{}`", self.key, expire_time),
            None => write!(f, "key: `{}`, permanent", self.key),
        }
    }
}

impl PeerKey {
    #[must_use]
    pub fn key(&self) -> Key {
        self.key.clone()
    }

    /// It returns the expiry time. For example, for the starting time for Unix Epoch
    /// (timestamp 0) it will return a `DateTime` whose string representation is
    /// `1970-01-01 00:00:00 UTC`.
    ///
    /// # Panics
    ///
    /// Will panic when the key timestamp overflows the internal i64 type.
    /// (this will naturally happen in 292.5 billion years)
    #[must_use]
    pub fn expiry_time(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        // We remove the fractions of seconds because we only store the seconds
        // in the database.
        self.valid_until
            .map(|valid_until| convert_from_timestamp_to_datetime_utc(Duration::from_secs(valid_until.as_secs())))
    }
}

/// A token used for authentication.
///
/// - It contains only ascii alphanumeric chars: lower and uppercase letters and
///   numbers.
/// - It's a 32-char string.
#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone, Display, Hash)]
pub struct Key(String);

impl Key {
    /// # Errors
    ///
    /// Will return an error is the string represents an invalid key.
    /// Valid keys can only contain 32 chars including 0-9, a-z and A-Z.
    pub fn new(value: &str) -> Result<Self, ParseKeyError> {
        if value.len() != AUTH_KEY_LENGTH {
            return Err(ParseKeyError::InvalidKeyLength);
        }

        if !value.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(ParseKeyError::InvalidChars);
        }

        Ok(Self(value.to_owned()))
    }

    /// It generates a random key.
    ///
    /// # Panics
    ///
    /// Will panic if the random number generator fails to generate a valid key.
    pub fn random() -> Self {
        let random_id: String = rng()
            .sample_iter(&Alphanumeric)
            .take(AUTH_KEY_LENGTH)
            .map(char::from)
            .collect();
        random_id.parse::<Key>().expect("Failed to generate a valid random key")
    }

    #[must_use]
    pub fn value(&self) -> &str {
        &self.0
    }
}

/// Error returned when a key cannot be parsed from a string.
///
/// ```text
/// use bittorrent_tracker_core::authentication::Key;
/// use std::str::FromStr;
///
/// let key_string = "YZSl4lMZupRuOpSRC3krIKR5BPB14nrJ";
/// let key = Key::from_str(key_string);
///
/// assert!(key.is_ok());
/// assert_eq!(key.unwrap().to_string(), key_string);
/// ```
///
/// If the string does not contains a valid key, the parser function will return
/// this error.
#[derive(Debug, Error)]
pub enum ParseKeyError {
    #[error("Invalid key length. Key must be have 32 chars")]
    InvalidKeyLength,

    #[error("Invalid chars for key. Key can only alphanumeric chars (0-9, a-z, A-Z)")]
    InvalidChars,
}

impl FromStr for Key {
    type Err = ParseKeyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Key::new(s)?;
        Ok(Self(s.to_string()))
    }
}

#[cfg(test)]
mod tests {

    mod key {
        use std::str::FromStr;

        use crate::authentication::Key;

        #[test]
        fn should_be_parsed_from_an_string() {
            let key_string = "YZSl4lMZupRuOpSRC3krIKR5BPB14nrJ";
            let key = Key::from_str(key_string);

            assert!(key.is_ok());
            assert_eq!(key.unwrap().to_string(), key_string);
        }

        #[test]
        fn should_be_generated_randomly() {
            let _key = Key::random();
        }

        #[test]
        fn length_should_be_32() {
            let key = Key::new("");
            assert!(key.is_err());

            let string_longer_than_32 = "012345678901234567890123456789012"; // DevSkim: ignore  DS173237
            let key = Key::new(string_longer_than_32);
            assert!(key.is_err());
        }

        #[test]
        fn should_only_include_alphanumeric_chars() {
            let key = Key::new("%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%%");
            assert!(key.is_err());
        }

        #[test]
        fn should_return_a_reference_to_the_inner_string() {
            let key = Key::new("YZSl4lMZupRuOpSRC3krIKR5BPB14nrJ").unwrap(); // DevSkim: ignore  DS173237

            assert_eq!(key.value(), "YZSl4lMZupRuOpSRC3krIKR5BPB14nrJ"); // DevSkim: ignore  DS173237
        }
    }

    mod peer_key {

        use std::time::Duration;

        use crate::authentication::key::peer_key::{Key, PeerKey};

        #[test]
        fn could_have_an_expiration_time() {
            let expiring_key = PeerKey {
                key: Key::random(),
                valid_until: Some(Duration::from_secs(100)),
            };

            assert_eq!(expiring_key.expiry_time().unwrap().to_string(), "1970-01-01 00:01:40 UTC");
        }

        #[test]
        fn could_be_permanent() {
            let permanent_key = PeerKey {
                key: Key::random(),
                valid_until: None,
            };

            assert_eq!(permanent_key.expiry_time(), None);
        }

        mod expiring {
            use std::time::Duration;

            use crate::authentication::key::peer_key::{Key, PeerKey};

            #[test]
            fn should_be_displayed_when_it_is_expiring() {
                let expiring_key = PeerKey {
                    key: Key::random(),
                    valid_until: Some(Duration::from_secs(100)),
                };

                assert_eq!(
                    expiring_key.to_string(),
                    format!("key: `{}`, valid until `1970-01-01 00:01:40 UTC`", expiring_key.key) // cspell:disable-line
                );
            }
        }

        mod permanent {

            use crate::authentication::key::peer_key::{Key, PeerKey};

            #[test]
            fn should_be_displayed_when_it_is_permanent() {
                let permanent_key = PeerKey {
                    key: Key::random(),
                    valid_until: None,
                };

                assert_eq!(
                    permanent_key.to_string(),
                    format!("key: `{}`, permanent", permanent_key.key) // cspell:disable-line
                );
            }
        }
    }
}
