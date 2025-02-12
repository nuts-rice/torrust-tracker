use std::sync::Arc;

use bittorrent_primitives::info_hash::InfoHash;

use super::repository::in_memory::InMemoryWhitelist;
use super::repository::persisted::DatabaseWhitelist;
use crate::databases;

/// It handles the list of allowed torrents. Only for listed trackers.
pub struct WhitelistManager {
    /// The in-memory list of allowed torrents.
    in_memory_whitelist: Arc<InMemoryWhitelist>,

    /// The persisted list of allowed torrents.
    database_whitelist: Arc<DatabaseWhitelist>,
}

impl WhitelistManager {
    #[must_use]
    pub fn new(database_whitelist: Arc<DatabaseWhitelist>, in_memory_whitelist: Arc<InMemoryWhitelist>) -> Self {
        Self {
            in_memory_whitelist,
            database_whitelist,
        }
    }

    /// It adds a torrent to the whitelist.
    /// Adding torrents is not relevant to public trackers.
    ///
    /// # Errors
    ///
    /// Will return a `database::Error` if unable to add the `info_hash` into the whitelist database.
    pub async fn add_torrent_to_whitelist(&self, info_hash: &InfoHash) -> Result<(), databases::error::Error> {
        self.database_whitelist.add(info_hash)?;
        self.in_memory_whitelist.add(info_hash).await;
        Ok(())
    }

    /// It removes a torrent from the whitelist.
    /// Removing torrents is not relevant to public trackers.
    ///
    /// # Errors
    ///
    /// Will return a `database::Error` if unable to remove the `info_hash` from the whitelist database.
    pub async fn remove_torrent_from_whitelist(&self, info_hash: &InfoHash) -> Result<(), databases::error::Error> {
        self.database_whitelist.remove(info_hash)?;
        self.in_memory_whitelist.remove(info_hash).await;
        Ok(())
    }

    /// It loads the whitelist from the database.
    ///
    /// # Errors
    ///
    /// Will return a `database::Error` if unable to load the list whitelisted `info_hash`s from the database.
    pub async fn load_whitelist_from_database(&self) -> Result<(), databases::error::Error> {
        let whitelisted_torrents_from_database = self.database_whitelist.load_from_database()?;

        self.in_memory_whitelist.clear().await;

        for info_hash in whitelisted_torrents_from_database {
            let _: bool = self.in_memory_whitelist.add(&info_hash).await;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use std::sync::Arc;

    use torrust_tracker_configuration::Core;

    use crate::databases::setup::initialize_database;
    use crate::databases::Database;
    use crate::test_helpers::tests::ephemeral_configuration_for_listed_tracker;
    use crate::whitelist::manager::WhitelistManager;
    use crate::whitelist::repository::in_memory::InMemoryWhitelist;
    use crate::whitelist::repository::persisted::DatabaseWhitelist;

    struct WhitelistManagerDeps {
        pub _database: Arc<Box<dyn Database>>,
        pub database_whitelist: Arc<DatabaseWhitelist>,
        pub in_memory_whitelist: Arc<InMemoryWhitelist>,
    }

    fn initialize_whitelist_manager_for_whitelisted_tracker() -> (Arc<WhitelistManager>, Arc<WhitelistManagerDeps>) {
        let config = ephemeral_configuration_for_listed_tracker();
        initialize_whitelist_manager_and_deps(&config)
    }

    fn initialize_whitelist_manager_and_deps(config: &Core) -> (Arc<WhitelistManager>, Arc<WhitelistManagerDeps>) {
        let database = initialize_database(config);
        let database_whitelist = Arc::new(DatabaseWhitelist::new(database.clone()));
        let in_memory_whitelist = Arc::new(InMemoryWhitelist::default());

        let whitelist_manager = Arc::new(WhitelistManager::new(database_whitelist.clone(), in_memory_whitelist.clone()));

        (
            whitelist_manager,
            Arc::new(WhitelistManagerDeps {
                _database: database,
                database_whitelist,
                in_memory_whitelist,
            }),
        )
    }

    mod configured_as_whitelisted {

        mod handling_the_torrent_whitelist {
            use crate::test_helpers::tests::sample_info_hash;
            use crate::whitelist::manager::tests::initialize_whitelist_manager_for_whitelisted_tracker;

            #[tokio::test]
            async fn it_should_add_a_torrent_to_the_whitelist() {
                let (whitelist_manager, services) = initialize_whitelist_manager_for_whitelisted_tracker();

                let info_hash = sample_info_hash();

                whitelist_manager.add_torrent_to_whitelist(&info_hash).await.unwrap();

                assert!(services.in_memory_whitelist.contains(&info_hash).await);
                assert!(services.database_whitelist.load_from_database().unwrap().contains(&info_hash));
            }

            #[tokio::test]
            async fn it_should_remove_a_torrent_from_the_whitelist() {
                let (whitelist_manager, services) = initialize_whitelist_manager_for_whitelisted_tracker();

                let info_hash = sample_info_hash();

                whitelist_manager.add_torrent_to_whitelist(&info_hash).await.unwrap();

                whitelist_manager.remove_torrent_from_whitelist(&info_hash).await.unwrap();

                assert!(!services.in_memory_whitelist.contains(&info_hash).await);
                assert!(!services.database_whitelist.load_from_database().unwrap().contains(&info_hash));
            }

            mod persistence {
                use crate::test_helpers::tests::sample_info_hash;
                use crate::whitelist::manager::tests::initialize_whitelist_manager_for_whitelisted_tracker;

                #[tokio::test]
                async fn it_should_load_the_whitelist_from_the_database() {
                    let (whitelist_manager, services) = initialize_whitelist_manager_for_whitelisted_tracker();

                    let info_hash = sample_info_hash();

                    services.database_whitelist.add(&info_hash).unwrap();

                    whitelist_manager.load_whitelist_from_database().await.unwrap();

                    assert!(services.in_memory_whitelist.contains(&info_hash).await);
                }
            }
        }
    }
}
