//! The `SQLite3` database driver.
use std::panic::Location;
use std::str::FromStr;

use bittorrent_primitives::info_hash::InfoHash;
use r2d2::Pool;
use r2d2_sqlite::rusqlite::params;
use r2d2_sqlite::rusqlite::types::Null;
use r2d2_sqlite::SqliteConnectionManager;
use torrust_tracker_primitives::{DurationSinceUnixEpoch, PersistentTorrents};

use super::{Database, Driver, Error};
use crate::authentication::{self, Key};

const DRIVER: Driver = Driver::Sqlite3;

pub struct Sqlite {
    pool: Pool<SqliteConnectionManager>,
}

impl Sqlite {
    /// It instantiates a new `SQLite3` database driver.
    ///
    /// Refer to [`databases::Database::new`](crate::core::databases::Database::new).
    ///
    /// # Errors
    ///
    /// Will return `r2d2::Error` if `db_path` is not able to create `SqLite` database.
    pub fn new(db_path: &str) -> Result<Self, Error> {
        let manager = SqliteConnectionManager::file(db_path);
        let pool = r2d2::Pool::builder().build(manager).map_err(|e| (e, DRIVER))?;

        Ok(Self { pool })
    }
}

impl Database for Sqlite {
    /// Refer to [`databases::Database::create_database_tables`](crate::core::databases::Database::create_database_tables).
    fn create_database_tables(&self) -> Result<(), Error> {
        let create_whitelist_table = "
        CREATE TABLE IF NOT EXISTS whitelist (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            info_hash TEXT NOT NULL UNIQUE
        );"
        .to_string();

        let create_torrents_table = "
        CREATE TABLE IF NOT EXISTS torrents (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            info_hash TEXT NOT NULL UNIQUE,
            completed INTEGER DEFAULT 0 NOT NULL
        );"
        .to_string();

        let create_keys_table = "
        CREATE TABLE IF NOT EXISTS keys (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            key TEXT NOT NULL UNIQUE,
            valid_until INTEGER
         );"
        .to_string();

        let conn = self.pool.get().map_err(|e| (e, DRIVER))?;

        conn.execute(&create_whitelist_table, [])?;
        conn.execute(&create_keys_table, [])?;
        conn.execute(&create_torrents_table, [])?;

        Ok(())
    }

    /// Refer to [`databases::Database::drop_database_tables`](crate::core::databases::Database::drop_database_tables).
    fn drop_database_tables(&self) -> Result<(), Error> {
        let drop_whitelist_table = "
        DROP TABLE whitelist;"
            .to_string();

        let drop_torrents_table = "
        DROP TABLE torrents;"
            .to_string();

        let drop_keys_table = "
        DROP TABLE keys;"
            .to_string();

        let conn = self.pool.get().map_err(|e| (e, DRIVER))?;

        conn.execute(&drop_whitelist_table, [])
            .and_then(|_| conn.execute(&drop_torrents_table, []))
            .and_then(|_| conn.execute(&drop_keys_table, []))?;

        Ok(())
    }

    /// Refer to [`databases::Database::load_persistent_torrents`](crate::core::databases::Database::load_persistent_torrents).
    fn load_persistent_torrents(&self) -> Result<PersistentTorrents, Error> {
        let conn = self.pool.get().map_err(|e| (e, DRIVER))?;

        let mut stmt = conn.prepare("SELECT info_hash, completed FROM torrents")?;

        let torrent_iter = stmt.query_map([], |row| {
            let info_hash_string: String = row.get(0)?;
            let info_hash = InfoHash::from_str(&info_hash_string).unwrap();
            let completed: u32 = row.get(1)?;
            Ok((info_hash, completed))
        })?;

        Ok(torrent_iter.filter_map(std::result::Result::ok).collect())
    }

    /// Refer to [`databases::Database::load_keys`](crate::core::databases::Database::load_keys).
    fn load_keys(&self) -> Result<Vec<authentication::PeerKey>, Error> {
        let conn = self.pool.get().map_err(|e| (e, DRIVER))?;

        let mut stmt = conn.prepare("SELECT key, valid_until FROM keys")?;

        let keys_iter = stmt.query_map([], |row| {
            let key: String = row.get(0)?;
            let opt_valid_until: Option<i64> = row.get(1)?;

            match opt_valid_until {
                Some(valid_until) => Ok(authentication::PeerKey {
                    key: key.parse::<Key>().unwrap(),
                    valid_until: Some(DurationSinceUnixEpoch::from_secs(valid_until.unsigned_abs())),
                }),
                None => Ok(authentication::PeerKey {
                    key: key.parse::<Key>().unwrap(),
                    valid_until: None,
                }),
            }
        })?;

        let keys: Vec<authentication::PeerKey> = keys_iter.filter_map(std::result::Result::ok).collect();

        Ok(keys)
    }

    /// Refer to [`databases::Database::load_whitelist`](crate::core::databases::Database::load_whitelist).
    fn load_whitelist(&self) -> Result<Vec<InfoHash>, Error> {
        let conn = self.pool.get().map_err(|e| (e, DRIVER))?;

        let mut stmt = conn.prepare("SELECT info_hash FROM whitelist")?;

        let info_hash_iter = stmt.query_map([], |row| {
            let info_hash: String = row.get(0)?;

            Ok(InfoHash::from_str(&info_hash).unwrap())
        })?;

        let info_hashes: Vec<InfoHash> = info_hash_iter.filter_map(std::result::Result::ok).collect();

        Ok(info_hashes)
    }

    /// Refer to [`databases::Database::save_persistent_torrent`](crate::core::databases::Database::save_persistent_torrent).
    fn save_persistent_torrent(&self, info_hash: &InfoHash, completed: u32) -> Result<(), Error> {
        let conn = self.pool.get().map_err(|e| (e, DRIVER))?;

        let insert = conn.execute(
            "INSERT INTO torrents (info_hash, completed) VALUES (?1, ?2) ON CONFLICT(info_hash) DO UPDATE SET completed = ?2",
            [info_hash.to_string(), completed.to_string()],
        )?;

        if insert == 0 {
            Err(Error::InsertFailed {
                location: Location::caller(),
                driver: DRIVER,
            })
        } else {
            Ok(())
        }
    }

    /// Refer to [`databases::Database::get_info_hash_from_whitelist`](crate::core::databases::Database::get_info_hash_from_whitelist).
    fn get_info_hash_from_whitelist(&self, info_hash: InfoHash) -> Result<Option<InfoHash>, Error> {
        let conn = self.pool.get().map_err(|e| (e, DRIVER))?;

        let mut stmt = conn.prepare("SELECT info_hash FROM whitelist WHERE info_hash = ?")?;

        let mut rows = stmt.query([info_hash.to_hex_string()])?;

        let query = rows.next()?;

        Ok(query.map(|f| InfoHash::from_str(&f.get_unwrap::<_, String>(0)).unwrap()))
    }

    /// Refer to [`databases::Database::add_info_hash_to_whitelist`](crate::core::databases::Database::add_info_hash_to_whitelist).
    fn add_info_hash_to_whitelist(&self, info_hash: InfoHash) -> Result<usize, Error> {
        let conn = self.pool.get().map_err(|e| (e, DRIVER))?;

        let insert = conn.execute("INSERT INTO whitelist (info_hash) VALUES (?)", [info_hash.to_string()])?;

        if insert == 0 {
            Err(Error::InsertFailed {
                location: Location::caller(),
                driver: DRIVER,
            })
        } else {
            Ok(insert)
        }
    }

    /// Refer to [`databases::Database::remove_info_hash_from_whitelist`](crate::core::databases::Database::remove_info_hash_from_whitelist).
    fn remove_info_hash_from_whitelist(&self, info_hash: InfoHash) -> Result<usize, Error> {
        let conn = self.pool.get().map_err(|e| (e, DRIVER))?;

        let deleted = conn.execute("DELETE FROM whitelist WHERE info_hash = ?", [info_hash.to_string()])?;

        if deleted == 1 {
            // should only remove a single record.
            Ok(deleted)
        } else {
            Err(Error::DeleteFailed {
                location: Location::caller(),
                error_code: deleted,
                driver: DRIVER,
            })
        }
    }

    /// Refer to [`databases::Database::get_key_from_keys`](crate::core::databases::Database::get_key_from_keys).
    fn get_key_from_keys(&self, key: &Key) -> Result<Option<authentication::PeerKey>, Error> {
        let conn = self.pool.get().map_err(|e| (e, DRIVER))?;

        let mut stmt = conn.prepare("SELECT key, valid_until FROM keys WHERE key = ?")?;

        let mut rows = stmt.query([key.to_string()])?;

        let key = rows.next()?;

        Ok(key.map(|f| {
            let valid_until: Option<i64> = f.get(1).unwrap();
            let key: String = f.get(0).unwrap();

            match valid_until {
                Some(valid_until) => authentication::PeerKey {
                    key: key.parse::<Key>().unwrap(),
                    valid_until: Some(DurationSinceUnixEpoch::from_secs(valid_until.unsigned_abs())),
                },
                None => authentication::PeerKey {
                    key: key.parse::<Key>().unwrap(),
                    valid_until: None,
                },
            }
        }))
    }

    /// Refer to [`databases::Database::add_key_to_keys`](crate::core::databases::Database::add_key_to_keys).
    fn add_key_to_keys(&self, auth_key: &authentication::PeerKey) -> Result<usize, Error> {
        let conn = self.pool.get().map_err(|e| (e, DRIVER))?;

        let insert = match auth_key.valid_until {
            Some(valid_until) => conn.execute(
                "INSERT INTO keys (key, valid_until) VALUES (?1, ?2)",
                [auth_key.key.to_string(), valid_until.as_secs().to_string()],
            )?,
            None => conn.execute(
                "INSERT INTO keys (key, valid_until) VALUES (?1, ?2)",
                params![auth_key.key.to_string(), Null],
            )?,
        };

        if insert == 0 {
            Err(Error::InsertFailed {
                location: Location::caller(),
                driver: DRIVER,
            })
        } else {
            Ok(insert)
        }
    }

    /// Refer to [`databases::Database::remove_key_from_keys`](crate::core::databases::Database::remove_key_from_keys).
    fn remove_key_from_keys(&self, key: &Key) -> Result<usize, Error> {
        let conn = self.pool.get().map_err(|e| (e, DRIVER))?;

        let deleted = conn.execute("DELETE FROM keys WHERE key = ?", [key.to_string()])?;

        if deleted == 1 {
            // should only remove a single record.
            Ok(deleted)
        } else {
            Err(Error::DeleteFailed {
                location: Location::caller(),
                error_code: deleted,
                driver: DRIVER,
            })
        }
    }
}

#[cfg(test)]
mod tests {

    mod the_sqlite_driver {
        use torrust_tracker_configuration::Core;
        use torrust_tracker_test_helpers::configuration::ephemeral_sqlite_database;

        use crate::databases::driver::sqlite::Sqlite;
        use crate::databases::Database;

        fn initialize_driver_and_database() -> Sqlite {
            let config = ephemeral_configuration();
            let driver = Sqlite::new(&config.database.path).unwrap();
            driver.create_database_tables().unwrap();
            driver
        }

        fn ephemeral_configuration() -> Core {
            let mut config = Core::default();
            let temp_file = ephemeral_sqlite_database();
            temp_file.to_str().unwrap().clone_into(&mut config.database.path);
            config
        }

        mod handling_torrent_persistence {

            use crate::core_tests::sample_info_hash;
            use crate::databases::driver::sqlite::tests::the_sqlite_driver::initialize_driver_and_database;
            use crate::databases::Database;

            #[test]
            fn it_should_save_and_load_persistent_torrents() {
                let driver = initialize_driver_and_database();

                let infohash = sample_info_hash();

                let number_of_downloads = 1;

                driver.save_persistent_torrent(&infohash, number_of_downloads).unwrap();

                let torrents = driver.load_persistent_torrents().unwrap();

                assert_eq!(torrents.len(), 1);
                assert_eq!(torrents.get(&infohash), Some(number_of_downloads).as_ref());
            }
        }

        mod handling_authentication_keys {
            use std::time::Duration;

            use crate::authentication::key::{generate_key, generate_permanent_key};
            use crate::databases::driver::sqlite::tests::the_sqlite_driver::initialize_driver_and_database;
            use crate::databases::Database;

            #[test]
            fn it_should_save_and_load_permanent_authentication_keys() {
                let driver = initialize_driver_and_database();

                // Add a new permanent key
                let peer_key = generate_permanent_key();
                driver.add_key_to_keys(&peer_key).unwrap();

                // Get the key back
                let stored_peer_key = driver.get_key_from_keys(&peer_key.key()).unwrap().unwrap();

                assert_eq!(stored_peer_key, peer_key);
            }
            #[test]
            fn it_should_save_and_load_expiring_authentication_keys() {
                let driver = initialize_driver_and_database();

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

            #[test]
            fn it_should_remove_an_authentication_key() {
                let driver = initialize_driver_and_database();

                let peer_key = generate_key(None);

                // Add a new key
                driver.add_key_to_keys(&peer_key).unwrap();

                // Remove the key
                driver.remove_key_from_keys(&peer_key.key()).unwrap();

                assert!(driver.get_key_from_keys(&peer_key.key()).unwrap().is_none());
            }
        }
    }
}
