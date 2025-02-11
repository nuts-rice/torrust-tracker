use std::sync::Arc;

use bittorrent_primitives::info_hash::InfoHash;
use torrust_tracker_primitives::PersistentTorrents;

use crate::databases::error::Error;
use crate::databases::Database;

/// Torrent repository implementation that persists the torrents in a database.
///
/// Not all the torrent in-memory data is persisted. For now only some of the
/// torrent metrics are persisted.
pub struct DatabasePersistentTorrentRepository {
    /// A database driver implementation: [`Sqlite3`](crate::core::databases::sqlite)
    /// or [`MySQL`](crate::core::databases::mysql)
    database: Arc<Box<dyn Database>>,
}

impl DatabasePersistentTorrentRepository {
    #[must_use]
    pub fn new(database: &Arc<Box<dyn Database>>) -> DatabasePersistentTorrentRepository {
        Self {
            database: database.clone(),
        }
    }

    /// It loads the persistent torrents from the database.
    ///
    /// # Errors
    ///
    /// Will return a database `Err` if unable to load.
    pub(crate) fn load_all(&self) -> Result<PersistentTorrents, Error> {
        self.database.load_persistent_torrents()
    }

    /// It saves the persistent torrent into the database.
    ///
    /// # Errors
    ///
    /// Will return a database `Err` if unable to save.
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
