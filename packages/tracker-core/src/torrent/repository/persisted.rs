//! The repository that stored persistent torrents' data into the database.
use std::sync::Arc;

use bittorrent_primitives::info_hash::InfoHash;
use torrust_tracker_primitives::PersistentTorrents;

use crate::databases::error::Error;
use crate::databases::Database;

/// Torrent repository implementation that persists torrent metrics in a database.
///
/// This repository persists only a subset of the torrent data: the torrent
/// metrics, specifically the number of downloads (or completed counts) for each
/// torrent. It relies on a database driver (either `SQLite3` or `MySQL`) that
/// implements the [`Database`] trait to perform the actual persistence
/// operations.
///
/// # Note
///
/// Not all in-memory torrent data is persisted; only the aggregate metrics are
/// stored.
pub struct DatabasePersistentTorrentRepository {
    /// A shared reference to the database driver implementation.
    ///
    /// The driver must implement the [`Database`] trait. This allows for
    /// different underlying implementations (e.g., `SQLite3` or `MySQL`) to be
    /// used interchangeably.
    database: Arc<Box<dyn Database>>,
}

impl DatabasePersistentTorrentRepository {
    /// Creates a new instance of `DatabasePersistentTorrentRepository`.
    ///
    /// # Arguments
    ///
    /// * `database` - A shared reference to a boxed database driver
    ///   implementing the [`Database`] trait.
    ///
    /// # Returns
    ///
    /// A new `DatabasePersistentTorrentRepository` instance with a cloned
    /// reference to the provided database.
    #[must_use]
    pub fn new(database: &Arc<Box<dyn Database>>) -> DatabasePersistentTorrentRepository {
        Self {
            database: database.clone(),
        }
    }

    /// Loads all persistent torrent metrics from the database.
    ///
    /// This function retrieves the torrent metrics (e.g., download counts) from the persistent store
    /// and returns them as a [`PersistentTorrents`] map.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the underlying database query fails.
    pub(crate) fn load_all(&self) -> Result<PersistentTorrents, Error> {
        self.database.load_persistent_torrents()
    }

    /// Saves the persistent torrent metric into the database.
    ///
    /// This function stores or updates the download count for the torrent
    /// identified by the provided infohash.
    ///
    /// # Arguments
    ///
    /// * `info_hash` - The info hash of the torrent.
    /// * `downloaded` - The number of times the torrent has been downloaded.
    ///
    /// # Errors
    ///
    /// Returns an [`Error`] if the database operation fails.
    pub(crate) fn save(&self, info_hash: &InfoHash, downloaded: u32) -> Result<(), Error> {
        self.database.save_persistent_torrent(info_hash, downloaded)
    }
}

#[cfg(test)]
mod tests {

    use torrust_tracker_primitives::PersistentTorrents;

    use super::DatabasePersistentTorrentRepository;
    use crate::databases::setup::initialize_database;
    use crate::test_helpers::tests::{ephemeral_configuration, sample_info_hash, sample_info_hash_one, sample_info_hash_two};

    fn initialize_db_persistent_torrent_repository() -> DatabasePersistentTorrentRepository {
        let config = ephemeral_configuration();
        let database = initialize_database(&config);
        DatabasePersistentTorrentRepository::new(&database)
    }

    #[test]
    fn it_saves_the_numbers_of_downloads_for_a_torrent_into_the_database() {
        let repository = initialize_db_persistent_torrent_repository();

        let infohash = sample_info_hash();

        repository.save(&infohash, 1).unwrap();

        let torrents = repository.load_all().unwrap();

        assert_eq!(torrents.get(&infohash), Some(1).as_ref());
    }

    #[test]
    fn it_loads_the_numbers_of_downloads_for_all_torrents_from_the_database() {
        let repository = initialize_db_persistent_torrent_repository();

        let infohash_one = sample_info_hash_one();
        let infohash_two = sample_info_hash_two();

        repository.save(&infohash_one, 1).unwrap();
        repository.save(&infohash_two, 2).unwrap();

        let torrents = repository.load_all().unwrap();

        let mut expected_torrents = PersistentTorrents::new();
        expected_torrents.insert(infohash_one, 1);
        expected_torrents.insert(infohash_two, 2);

        assert_eq!(torrents, expected_torrents);
    }
}
