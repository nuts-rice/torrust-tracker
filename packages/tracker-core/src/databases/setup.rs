use std::sync::Arc;

use torrust_tracker_configuration::Core;

use super::driver::{self, Driver};
use super::Database;

/// # Panics
///
/// Will panic if database cannot be initialized.
#[must_use]
pub fn initialize_database(config: &Core) -> Arc<Box<dyn Database>> {
    let driver = match config.database.driver {
        torrust_tracker_configuration::Driver::Sqlite3 => Driver::Sqlite3,
        torrust_tracker_configuration::Driver::MySQL => Driver::MySQL,
    };

    Arc::new(driver::build(&driver, &config.database.path).expect("Database driver build failed."))
}
