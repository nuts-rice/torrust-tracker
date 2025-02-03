use std::sync::Arc;

use crate::authentication::key::{Key, PeerKey};
use crate::databases::{self, Database};

/// The database repository for the authentication keys.
pub struct DatabaseKeyRepository {
    database: Arc<Box<dyn Database>>,
}

impl DatabaseKeyRepository {
    #[must_use]
    pub fn new(database: &Arc<Box<dyn Database>>) -> Self {
        Self {
            database: database.clone(),
        }
    }

    /// It adds a new key to the database.
    ///
    /// # Errors
    ///
    /// Will return a `databases::error::Error` if unable to add the `auth_key` to the database.
    pub fn add(&self, peer_key: &PeerKey) -> Result<(), databases::error::Error> {
        self.database.add_key_to_keys(peer_key)?;
        Ok(())
    }

    /// It removes an key from the database.
    ///
    /// # Errors
    ///
    /// Will return a `database::Error` if unable to remove the `key` from the database.
    pub fn remove(&self, key: &Key) -> Result<(), databases::error::Error> {
        self.database.remove_key_from_keys(key)?;
        Ok(())
    }

    /// It loads all keys from the database.
    ///
    /// # Errors
    ///
    /// Will return a `database::Error` if unable to load the keys from the database.
    pub fn load_keys(&self) -> Result<Vec<PeerKey>, databases::error::Error> {
        let keys = self.database.load_keys()?;
        Ok(keys)
    }
}

#[cfg(test)]
mod tests {

    mod the_persisted_key_repository_should {

        use std::time::Duration;

        use torrust_tracker_configuration::Core;
        use torrust_tracker_test_helpers::configuration::ephemeral_sqlite_database;

        use crate::authentication::key::repository::persisted::DatabaseKeyRepository;
        use crate::authentication::{Key, PeerKey};
        use crate::databases::setup::initialize_database;

        fn ephemeral_configuration() -> Core {
            let mut config = Core::default();
            let temp_file = ephemeral_sqlite_database();
            temp_file.to_str().unwrap().clone_into(&mut config.database.path);
            config
        }

        #[test]
        fn persist_a_new_peer_key() {
            let configuration = ephemeral_configuration();

            let database = initialize_database(&configuration);

            let repository = DatabaseKeyRepository::new(&database);

            let peer_key = PeerKey {
                key: Key::new("YZSl4lMZupRuOpSRC3krIKR5BPB14nrJ").unwrap(),
                valid_until: Some(Duration::new(9999, 0)),
            };

            let result = repository.add(&peer_key);
            assert!(result.is_ok());

            let keys = repository.load_keys().unwrap();
            assert_eq!(keys, vec!(peer_key));
        }

        #[test]
        fn remove_a_persisted_peer_key() {
            let configuration = ephemeral_configuration();

            let database = initialize_database(&configuration);

            let repository = DatabaseKeyRepository::new(&database);

            let peer_key = PeerKey {
                key: Key::new("YZSl4lMZupRuOpSRC3krIKR5BPB14nrJ").unwrap(),
                valid_until: Some(Duration::new(9999, 0)),
            };

            let _unused = repository.add(&peer_key);

            let result = repository.remove(&peer_key.key);
            assert!(result.is_ok());

            let keys = repository.load_keys().unwrap();
            assert!(keys.is_empty());
        }

        #[test]
        fn load_all_persisted_peer_keys() {
            let configuration = ephemeral_configuration();

            let database = initialize_database(&configuration);

            let repository = DatabaseKeyRepository::new(&database);

            let peer_key = PeerKey {
                key: Key::new("YZSl4lMZupRuOpSRC3krIKR5BPB14nrJ").unwrap(),
                valid_until: Some(Duration::new(9999, 0)),
            };

            let _unused = repository.add(&peer_key);

            let keys = repository.load_keys().unwrap();

            assert_eq!(keys, vec!(peer_key));
        }
    }
}
