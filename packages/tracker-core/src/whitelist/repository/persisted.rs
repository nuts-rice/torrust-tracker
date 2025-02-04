use std::sync::Arc;

use bittorrent_primitives::info_hash::InfoHash;

use crate::databases::{self, Database};

/// The persisted list of allowed torrents.
pub struct DatabaseWhitelist {
    /// A database driver implementation: [`Sqlite3`](crate::core::databases::sqlite)
    /// or [`MySQL`](crate::core::databases::mysql)
    database: Arc<Box<dyn Database>>,
}

impl DatabaseWhitelist {
    #[must_use]
    pub fn new(database: Arc<Box<dyn Database>>) -> Self {
        Self { database }
    }

    /// It adds a torrent to the whitelist if it has not been whitelisted previously
    ///
    /// # Errors
    ///
    /// Will return a `database::Error` if unable to add the `info_hash` to the whitelist database.
    pub fn add(&self, info_hash: &InfoHash) -> Result<(), databases::error::Error> {
        let is_whitelisted = self.database.is_info_hash_whitelisted(*info_hash)?;

        if is_whitelisted {
            return Ok(());
        }

        self.database.add_info_hash_to_whitelist(*info_hash)?;

        Ok(())
    }

    /// It removes a torrent from the whitelist in the database.
    ///
    /// # Errors
    ///
    /// Will return a `database::Error` if unable to remove the `info_hash` from the whitelist database.
    pub fn remove(&self, info_hash: &InfoHash) -> Result<(), databases::error::Error> {
        let is_whitelisted = self.database.is_info_hash_whitelisted(*info_hash)?;

        if !is_whitelisted {
            return Ok(());
        }

        self.database.remove_info_hash_from_whitelist(*info_hash)?;

        Ok(())
    }

    /// It loads the whitelist from the database.
    ///
    /// # Errors
    ///
    /// Will return a `database::Error` if unable to load the list whitelisted `info_hash`s from the database.
    pub fn load_from_database(&self) -> Result<Vec<InfoHash>, databases::error::Error> {
        self.database.load_whitelist()
    }
}

#[cfg(test)]
mod tests {
    mod the_persisted_whitelist_repository {

        use torrust_tracker_configuration::Core;
        use torrust_tracker_test_helpers::configuration::ephemeral_sqlite_database;

        use crate::core_tests::sample_info_hash;
        use crate::databases::setup::initialize_database;
        use crate::whitelist::repository::persisted::DatabaseWhitelist;

        fn initialize_database_whitelist() -> DatabaseWhitelist {
            let configuration = ephemeral_configuration_for_listed_tracker();
            let database = initialize_database(&configuration);
            DatabaseWhitelist::new(database)
        }

        fn ephemeral_configuration_for_listed_tracker() -> Core {
            let mut config = Core {
                listed: true,
                ..Default::default()
            };

            let temp_file = ephemeral_sqlite_database();
            temp_file.to_str().unwrap().clone_into(&mut config.database.path);

            config
        }

        #[test]
        fn should_add_a_new_infohash_to_the_list() {
            let whitelist = initialize_database_whitelist();

            let infohash = sample_info_hash();

            let _result = whitelist.add(&infohash);

            assert_eq!(whitelist.load_from_database().unwrap(), vec!(infohash));
        }

        #[test]
        fn should_remove_a_infohash_from_the_list() {
            let whitelist = initialize_database_whitelist();

            let infohash = sample_info_hash();

            let _result = whitelist.add(&infohash);

            let _result = whitelist.remove(&infohash);

            assert_eq!(whitelist.load_from_database().unwrap(), vec!());
        }

        #[test]
        fn should_load_all_infohashes_from_the_database() {
            let whitelist = initialize_database_whitelist();

            let infohash = sample_info_hash();

            let _result = whitelist.add(&infohash);

            let result = whitelist.load_from_database().unwrap();

            assert_eq!(result, vec!(infohash));
        }

        #[test]
        fn should_not_add_the_same_infohash_to_the_list_twice() {
            let whitelist = initialize_database_whitelist();

            let infohash = sample_info_hash();

            let _result = whitelist.add(&infohash);
            let _result = whitelist.add(&infohash);

            assert_eq!(whitelist.load_from_database().unwrap(), vec!(infohash));
        }

        #[test]
        fn should_not_fail_removing_an_infohash_that_is_not_in_the_list() {
            let whitelist = initialize_database_whitelist();

            let infohash = sample_info_hash();

            let result = whitelist.remove(&infohash);

            assert!(result.is_ok());
        }
    }
}
