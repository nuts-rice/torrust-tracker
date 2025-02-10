//! Database driver factory.
//!
//! See [`databases::driver::build`](crate::core::databases::driver::build)
//! function for more information.
use mysql::Mysql;
use serde::{Deserialize, Serialize};
use sqlite::Sqlite;

use super::error::Error;
use super::Database;

/// The database management system used by the tracker.
///
/// Refer to:
///
/// - [Torrust Tracker Configuration](https://docs.rs/torrust-tracker-configuration).
/// - [Torrust Tracker](https://docs.rs/torrust-tracker).
///
/// For more information about persistence.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, derive_more::Display, Clone)]
pub enum Driver {
    /// The Sqlite3 database driver.
    Sqlite3,
    /// The `MySQL` database driver.
    MySQL,
}

/// It builds a new database driver.
///
/// Example for `SQLite3`:
///
/// ```text
/// use bittorrent_tracker_core::databases;
/// use bittorrent_tracker_core::databases::driver::Driver;
///
/// let db_driver = Driver::Sqlite3;
/// let db_path = "./storage/tracker/lib/database/sqlite3.db".to_string();
/// let database = databases::driver::build(&db_driver, &db_path);
/// ```
///
/// Example for `MySQL`:
///
/// ```text
/// use bittorrent_tracker_core::databases;
/// use bittorrent_tracker_core::databases::driver::Driver;
///
/// let db_driver = Driver::MySQL;
/// let db_path = "mysql://db_user:db_user_secret_password@mysql:3306/torrust_tracker".to_string();
/// let database = databases::driver::build(&db_driver, &db_path);
/// ```
///
/// Refer to the [configuration documentation](https://docs.rs/torrust-tracker-configuration)
/// for more information about the database configuration.
///
/// > **WARNING**: The driver instantiation runs database migrations.
///
/// # Errors
///
/// This function will return an error if unable to connect to the database.
///
/// # Panics
///
/// This function will panic if unable to create database tables.
pub mod mysql;
pub mod sqlite;

/// It builds a new database driver.
///
/// # Panics
///
/// Will panic if unable to create database tables.
///
/// # Errors
///
/// Will return `Error` if unable to build the driver.
pub fn build(driver: &Driver, db_path: &str) -> Result<Box<dyn Database>, Error> {
    let database: Box<dyn Database> = match driver {
        Driver::Sqlite3 => Box::new(Sqlite::new(db_path)?),
        Driver::MySQL => Box::new(Mysql::new(db_path)?),
    };

    database.create_database_tables().expect("Could not create database tables.");

    Ok(database)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use crate::databases::Database;

    pub async fn run_tests(driver: &Arc<Box<dyn Database>>) {
        // Since the interface is very simple and there are no conflicts between
        // tests, we share the same database. If we want to isolate the tests in
        // the future, we can create a new database for each test.

        create_database_tables(driver).await.unwrap();

        // todo: truncate tables otherwise they will increase in size over time.
        // That's not a problem on CI when the database is always newly created.

        handling_torrent_persistence::it_should_save_and_load_persistent_torrents(driver);

        // Permanent keys
        handling_authentication_keys::it_should_save_and_load_permanent_authentication_keys(driver);
        //handling_authentication_keys::it_should_remove_a_permanent_authentication_key(&driver);

        // Expiring keys
        handling_authentication_keys::it_should_save_and_load_expiring_authentication_keys(driver);
        //handling_authentication_keys::it_should_remove_an_expiring_authentication_key(&driver);

        driver.drop_database_tables().unwrap();
    }

    async fn create_database_tables(driver: &Arc<Box<dyn Database>>) -> Result<(), Box<dyn std::error::Error>> {
        for _ in 0..5 {
            if driver.create_database_tables().is_ok() {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
        Err("MySQL is not ready after retries.".into())
    }

    mod handling_torrent_persistence {

        use std::sync::Arc;

        use crate::core_tests::sample_info_hash;
        use crate::databases::Database;

        pub fn it_should_save_and_load_persistent_torrents(driver: &Arc<Box<dyn Database>>) {
            let infohash = sample_info_hash();

            let number_of_downloads = 1;

            driver.save_persistent_torrent(&infohash, number_of_downloads).unwrap();

            let torrents = driver.load_persistent_torrents().unwrap();

            assert_eq!(torrents.len(), 1);
            assert_eq!(torrents.get(&infohash), Some(number_of_downloads).as_ref());
        }
    }

    mod handling_authentication_keys {

        use std::sync::Arc;
        use std::time::Duration;

        use crate::authentication::key::{generate_key, generate_permanent_key};
        use crate::databases::Database;

        pub fn it_should_save_and_load_permanent_authentication_keys(driver: &Arc<Box<dyn Database>>) {
            // Add a new permanent key
            let peer_key = generate_permanent_key();
            driver.add_key_to_keys(&peer_key).unwrap();

            // Get the key back
            let stored_peer_key = driver.get_key_from_keys(&peer_key.key()).unwrap().unwrap();

            assert_eq!(stored_peer_key, peer_key);
        }

        pub fn it_should_save_and_load_expiring_authentication_keys(driver: &Arc<Box<dyn Database>>) {
            // Add a new expiring key
            let peer_key = generate_key(Some(Duration::from_secs(120)));
            driver.add_key_to_keys(&peer_key).unwrap();

            // Get the key back
            let stored_peer_key = driver.get_key_from_keys(&peer_key.key()).unwrap().unwrap();

            /* todo:

            The expiration time recovered from the database is not the same
            as the one we set. It includes a small offset (nanoseconds).

            left: PeerKey { key: Key("7HP1NslpuQn6kLVAgAF4nFpnZNSQ4hrx"), valid_until: Some(1739182308s) }
            right: PeerKey { key: Key("7HP1NslpuQn6kLVAgAF4nFpnZNSQ4hrx"), valid_until: Some(1739182308.603691299s)

            */

            assert_eq!(stored_peer_key.key(), peer_key.key());
            assert_eq!(
                stored_peer_key.valid_until.unwrap().as_secs(),
                peer_key.valid_until.unwrap().as_secs()
            );
        }

        /*pub fn it_should_remove_a_permanent_authentication_key(driver: &Arc<Box<dyn Database>>) {
            let peer_key = generate_permanent_key();

            // Add a new key
            driver.add_key_to_keys(&peer_key).unwrap();

            // Remove the key
            driver.remove_key_from_keys(&peer_key.key()).unwrap();

            assert!(driver.get_key_from_keys(&peer_key.key()).unwrap().is_none());
        }*/

        /*pub fn it_should_remove_an_expiring_authentication_key(driver: &Arc<Box<dyn Database>>) {
            let peer_key = generate_key(Some(Duration::from_secs(120)));

            // Add a new key
            driver.add_key_to_keys(&peer_key).unwrap();

            // Remove the key
            driver.remove_key_from_keys(&peer_key.key()).unwrap();

            assert!(driver.get_key_from_keys(&peer_key.key()).unwrap().is_none());
        }*/
    }
}
