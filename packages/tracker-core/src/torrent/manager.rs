use std::sync::Arc;
use std::time::Duration;

use torrust_tracker_clock::clock::Time;
use torrust_tracker_configuration::Core;

use super::repository::in_memory::InMemoryTorrentRepository;
use super::repository::persisted::DatabasePersistentTorrentRepository;
use crate::{databases, CurrentClock};

pub struct TorrentsManager {
    /// The tracker configuration.
    config: Core,

    /// The in-memory torrents repository.
    in_memory_torrent_repository: Arc<InMemoryTorrentRepository>,

    /// The persistent torrents repository.
    db_torrent_repository: Arc<DatabasePersistentTorrentRepository>,
}

impl TorrentsManager {
    #[must_use]
    pub fn new(
        config: &Core,
        in_memory_torrent_repository: &Arc<InMemoryTorrentRepository>,
        db_torrent_repository: &Arc<DatabasePersistentTorrentRepository>,
    ) -> Self {
        Self {
            config: config.clone(),
            in_memory_torrent_repository: in_memory_torrent_repository.clone(),
            db_torrent_repository: db_torrent_repository.clone(),
        }
    }

    /// It loads the torrents from database into memory. It only loads the
    /// torrent entry list with the number of seeders for each torrent. Peers
    /// data is not persisted.
    ///
    /// # Errors
    ///
    /// Will return a `database::Error` if unable to load the list of `persistent_torrents` from the database.
    pub fn load_torrents_from_database(&self) -> Result<(), databases::error::Error> {
        let persistent_torrents = self.db_torrent_repository.load_all()?;

        self.in_memory_torrent_repository.import_persistent(&persistent_torrents);

        Ok(())
    }

    /// Remove inactive peers and (optionally) peerless torrents.
    pub fn cleanup_torrents(&self) {
        let current_cutoff = CurrentClock::now_sub(&Duration::from_secs(u64::from(self.config.tracker_policy.max_peer_timeout)))
            .unwrap_or_default();

        self.in_memory_torrent_repository.remove_inactive_peers(current_cutoff);

        if self.config.tracker_policy.remove_peerless_torrents {
            self.in_memory_torrent_repository
                .remove_peerless_torrents(&self.config.tracker_policy);
        }
    }
}

#[cfg(test)]
mod tests {

    use std::sync::Arc;

    use torrust_tracker_configuration::Core;
    use torrust_tracker_torrent_repository::entry::EntrySync;

    use super::{DatabasePersistentTorrentRepository, TorrentsManager};
    use crate::core_tests::{ephemeral_configuration, sample_info_hash};
    use crate::databases::setup::initialize_database;
    use crate::torrent::repository::in_memory::InMemoryTorrentRepository;

    struct TorrentsManagerDeps {
        config: Arc<Core>,
        in_memory_torrent_repository: Arc<InMemoryTorrentRepository>,
        database_persistent_torrent_repository: Arc<DatabasePersistentTorrentRepository>,
    }

    fn initialize_torrents_manager() -> (Arc<TorrentsManager>, Arc<TorrentsManagerDeps>) {
        let config = ephemeral_configuration();
        initialize_torrents_manager_with(config.clone())
    }

    fn initialize_torrents_manager_with(config: Core) -> (Arc<TorrentsManager>, Arc<TorrentsManagerDeps>) {
        let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());
        let database = initialize_database(&config);
        let database_persistent_torrent_repository = Arc::new(DatabasePersistentTorrentRepository::new(&database));

        let torrents_manager = Arc::new(TorrentsManager::new(
            &config,
            &in_memory_torrent_repository,
            &database_persistent_torrent_repository,
        ));

        (
            torrents_manager,
            Arc::new(TorrentsManagerDeps {
                config: Arc::new(config),
                in_memory_torrent_repository,
                database_persistent_torrent_repository,
            }),
        )
    }

    #[test]
    fn it_should_load_the_numbers_of_downloads_for_all_torrents_from_the_database() {
        let (torrents_manager, services) = initialize_torrents_manager();

        let infohash = sample_info_hash();

        services.database_persistent_torrent_repository.save(&infohash, 1).unwrap();

        torrents_manager.load_torrents_from_database().unwrap();

        assert_eq!(
            services
                .in_memory_torrent_repository
                .get(&infohash)
                .unwrap()
                .get_swarm_metadata()
                .downloaded,
            1
        );
    }

    mod cleaning_torrents {
        use std::ops::Add;
        use std::sync::Arc;
        use std::time::Duration;

        use bittorrent_primitives::info_hash::InfoHash;
        use torrust_tracker_clock::clock::stopped::Stopped;
        use torrust_tracker_clock::clock::{self};
        use torrust_tracker_primitives::DurationSinceUnixEpoch;

        use crate::core_tests::{ephemeral_configuration, sample_info_hash, sample_peer};
        use crate::torrent::manager::tests::{initialize_torrents_manager, initialize_torrents_manager_with};
        use crate::torrent::repository::in_memory::InMemoryTorrentRepository;

        #[test]
        fn it_should_remove_peers_that_have_not_been_updated_after_a_cutoff_time() {
            let (torrents_manager, services) = initialize_torrents_manager();

            let infohash = sample_info_hash();

            clock::Stopped::local_set(&Duration::from_secs(0));

            // Add a peer to the torrent
            let mut peer = sample_peer();
            peer.updated = DurationSinceUnixEpoch::new(0, 0);
            let () = services.in_memory_torrent_repository.upsert_peer(&infohash, &peer);

            // Simulate the time has passed 1 second more than the max peer timeout.
            clock::Stopped::local_add(&Duration::from_secs(
                (services.config.tracker_policy.max_peer_timeout + 1).into(),
            ))
            .unwrap();

            torrents_manager.cleanup_torrents();

            assert!(services.in_memory_torrent_repository.get(&infohash).is_none());
        }

        fn add_a_peerless_torrent(infohash: &InfoHash, in_memory_torrent_repository: &Arc<InMemoryTorrentRepository>) {
            // Add a peer to the torrent
            let mut peer = sample_peer();
            peer.updated = DurationSinceUnixEpoch::new(0, 0);
            let () = in_memory_torrent_repository.upsert_peer(infohash, &peer);

            // Remove the peer. The torrent is now peerless.
            in_memory_torrent_repository.remove_inactive_peers(peer.updated.add(Duration::from_secs(1)));
        }

        #[test]
        fn it_should_remove_torrents_that_have_no_peers_when_it_is_configured_to_do_so() {
            let mut config = ephemeral_configuration();
            config.tracker_policy.remove_peerless_torrents = true;

            let (torrents_manager, services) = initialize_torrents_manager_with(config);

            let infohash = sample_info_hash();

            add_a_peerless_torrent(&infohash, &services.in_memory_torrent_repository);

            torrents_manager.cleanup_torrents();

            assert!(services.in_memory_torrent_repository.get(&infohash).is_none());
        }

        #[test]
        fn it_should_retain_peerless_torrents_when_it_is_configured_to_do_so() {
            let mut config = ephemeral_configuration();
            config.tracker_policy.remove_peerless_torrents = false;

            let (torrents_manager, services) = initialize_torrents_manager_with(config);

            let infohash = sample_info_hash();

            add_a_peerless_torrent(&infohash, &services.in_memory_torrent_repository);

            torrents_manager.cleanup_torrents();

            assert!(services.in_memory_torrent_repository.get(&infohash).is_some());
        }
    }
}
