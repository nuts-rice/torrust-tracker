//! Errors returned by the core tracker.
use std::panic::Location;

use bittorrent_primitives::info_hash::InfoHash;
use torrust_tracker_located_error::LocatedError;

use super::authentication::key::ParseKeyError;
use super::databases;

/// Whitelist errors returned by the core tracker.
#[derive(thiserror::Error, Debug, Clone)]
pub enum WhitelistError {
    #[error("The torrent: {info_hash}, is not whitelisted, {location}")]
    TorrentNotWhitelisted {
        info_hash: InfoHash,
        location: &'static Location<'static>,
    },
}

/// Peers keys errors returned by the core tracker.
#[allow(clippy::module_name_repetitions)]
#[derive(thiserror::Error, Debug, Clone)]
pub enum PeerKeyError {
    #[error("Invalid peer key duration: {seconds_valid:?}, is not valid")]
    DurationOverflow { seconds_valid: u64 },

    #[error("Invalid key: {key}")]
    InvalidKey {
        key: String,
        source: LocatedError<'static, ParseKeyError>,
    },

    #[error("Can't persist key: {source}")]
    DatabaseError {
        source: LocatedError<'static, databases::error::Error>,
    },
}

#[cfg(test)]
mod tests {

    mod whitelist_error {

        use crate::core_tests::sample_info_hash;
        use crate::error::WhitelistError;

        #[test]
        fn torrent_not_whitelisted() {
            let err = WhitelistError::TorrentNotWhitelisted {
                info_hash: sample_info_hash(),
                location: std::panic::Location::caller(),
            };

            let err_msg = format!("{err}");

            assert!(
                err_msg.contains(&format!("The torrent: {}, is not whitelisted", sample_info_hash())),
                "Error message did not contain expected text: {err_msg}"
            );
        }
    }

    mod peer_key_error {
        use torrust_tracker_located_error::Located;

        use crate::databases::driver::Driver;
        use crate::error::PeerKeyError;
        use crate::{authentication, databases};

        #[test]
        fn duration_overflow() {
            let seconds_valid = 100;

            let err = PeerKeyError::DurationOverflow { seconds_valid };

            let err_msg = format!("{err}");

            assert!(
                err_msg.contains(&format!("Invalid peer key duration: {seconds_valid}")),
                "Error message did not contain expected text: {err_msg}"
            );
        }

        #[test]
        fn parsing_from_string() {
            let err = authentication::key::ParseKeyError::InvalidKeyLength;

            let err = PeerKeyError::InvalidKey {
                key: "INVALID KEY".to_string(),
                source: Located(err).into(),
            };

            let err_msg = format!("{err}");

            assert!(
                err_msg.contains(&"Invalid key: INVALID KEY".to_string()),
                "Error message did not contain expected text: {err_msg}"
            );
        }

        #[test]
        fn persisting_into_database() {
            let err = databases::error::Error::InsertFailed {
                location: std::panic::Location::caller(),
                driver: Driver::Sqlite3,
            };

            let err = PeerKeyError::DatabaseError {
                source: Located(err).into(),
            };

            let err_msg = format!("{err}");

            assert!(
                err_msg.contains(&"Can't persist key".to_string()),
                "Error message did not contain expected text: {err}"
            );
        }
    }
}
