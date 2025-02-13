//! Initializes the whitelist manager.
//!
//! This module provides functions to set up the `WhitelistManager`, which is responsible
//! for managing whitelisted torrents in both the in-memory and persistent database repositories.
use std::sync::Arc;

use super::manager::WhitelistManager;
use super::repository::in_memory::InMemoryWhitelist;
use super::repository::persisted::DatabaseWhitelist;
use crate::databases::Database;

/// Initializes the `WhitelistManager` by combining in-memory and database
/// repositories.
///
/// The `WhitelistManager` handles the operations related to whitelisted
/// torrents, such as adding, removing, and verifying torrents in the whitelist.
/// It operates with:
///
/// 1. **In-Memory Whitelist:** Provides fast, runtime-based access to
///    whitelisted torrents.
/// 2. **Database Whitelist:** Ensures persistent storage of the whitelist data.
///
/// # Arguments
///
/// * `database` - An `Arc<Box<dyn Database>>` representing the database connection,
///   sed for persistent whitelist storage.
/// * `in_memory_whitelist` - An `Arc<InMemoryWhitelist>` representing the in-memory
///   whitelist repository for fast access.
///
/// # Returns
///
/// An `Arc<WhitelistManager>` instance that manages both the in-memory and database
/// whitelist repositories.
#[must_use]
pub fn initialize_whitelist_manager(
    database: Arc<Box<dyn Database>>,
    in_memory_whitelist: Arc<InMemoryWhitelist>,
) -> Arc<WhitelistManager> {
    let database_whitelist = Arc::new(DatabaseWhitelist::new(database));
    Arc::new(WhitelistManager::new(database_whitelist, in_memory_whitelist))
}
