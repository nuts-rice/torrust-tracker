//! Errors returned by the core tracker.
use std::panic::Location;

use bittorrent_primitives::info_hash::InfoHash;
use torrust_tracker_located_error::LocatedError;

use super::authentication::key::ParseKeyError;
use super::databases;

/// Authorization errors returned by the core tracker.
#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
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
