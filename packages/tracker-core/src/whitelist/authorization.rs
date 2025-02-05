use std::panic::Location;
use std::sync::Arc;

use bittorrent_primitives::info_hash::InfoHash;
use torrust_tracker_configuration::Core;
use tracing::instrument;

use super::repository::in_memory::InMemoryWhitelist;
use crate::error::Error;

pub struct WhitelistAuthorization {
    /// Core tracker configuration.
    config: Core,

    /// The in-memory list of allowed torrents.
    in_memory_whitelist: Arc<InMemoryWhitelist>,
}

impl WhitelistAuthorization {
    /// Creates a new authorization instance.
    pub fn new(config: &Core, in_memory_whitelist: &Arc<InMemoryWhitelist>) -> Self {
        Self {
            config: config.clone(),
            in_memory_whitelist: in_memory_whitelist.clone(),
        }
    }

    /// It returns true if the torrent is authorized.
    ///
    /// # Errors
    ///
    /// Will return an error if the tracker is running in `listed` mode
    /// and the infohash is not whitelisted.
    #[instrument(skip(self, info_hash), err)]
    pub async fn authorize(&self, info_hash: &InfoHash) -> Result<(), Error> {
        if !self.is_listed() {
            return Ok(());
        }

        if self.is_info_hash_whitelisted(info_hash).await {
            return Ok(());
        }

        Err(Error::TorrentNotWhitelisted {
            info_hash: *info_hash,
            location: Location::caller(),
        })
    }

    /// Returns `true` is the tracker is in listed mode.
    fn is_listed(&self) -> bool {
        self.config.listed
    }

    /// It checks if a torrent is whitelisted.
    async fn is_info_hash_whitelisted(&self, info_hash: &InfoHash) -> bool {
        self.in_memory_whitelist.contains(info_hash).await
    }
}

#[cfg(test)]
mod tests {

    mod the_whitelist_authorization_for_announce_and_scrape_actions {
        use std::sync::Arc;

        use torrust_tracker_configuration::Core;

        use crate::whitelist::authorization::WhitelistAuthorization;
        use crate::whitelist::repository::in_memory::InMemoryWhitelist;

        fn initialize_whitelist_authorization_with(config: &Core) -> Arc<WhitelistAuthorization> {
            let (whitelist_authorization, _in_memory_whitelist) =
                initialize_whitelist_authorization_and_dependencies_with(config);
            whitelist_authorization
        }

        fn initialize_whitelist_authorization_and_dependencies_with(
            config: &Core,
        ) -> (Arc<WhitelistAuthorization>, Arc<InMemoryWhitelist>) {
            let in_memory_whitelist = Arc::new(InMemoryWhitelist::default());
            let whitelist_authorization = Arc::new(WhitelistAuthorization::new(config, &in_memory_whitelist.clone()));

            (whitelist_authorization, in_memory_whitelist)
        }

        mod when_the_tacker_is_configured_as_listed {

            use torrust_tracker_configuration::Core;

            use crate::core_tests::sample_info_hash;
            use crate::error::Error;
            use crate::whitelist::authorization::tests::the_whitelist_authorization_for_announce_and_scrape_actions::{
                initialize_whitelist_authorization_and_dependencies_with, initialize_whitelist_authorization_with,
            };

            fn configuration_for_listed_tracker() -> Core {
                Core {
                    listed: true,
                    ..Default::default()
                }
            }

            #[tokio::test]
            async fn should_authorize_a_whitelisted_infohash() {
                let (whitelist_authorization, in_memory_whitelist) =
                    initialize_whitelist_authorization_and_dependencies_with(&configuration_for_listed_tracker());

                let info_hash = sample_info_hash();

                let _unused = in_memory_whitelist.add(&info_hash).await;

                let result = whitelist_authorization.authorize(&info_hash).await;

                assert!(result.is_ok());
            }

            #[tokio::test]
            async fn should_not_authorize_a_non_whitelisted_infohash() {
                let whitelist_authorization = initialize_whitelist_authorization_with(&configuration_for_listed_tracker());

                let result = whitelist_authorization.authorize(&sample_info_hash()).await;

                assert!(matches!(result.unwrap_err(), Error::TorrentNotWhitelisted { .. }));
            }
        }

        mod when_the_tacker_is_not_configured_as_listed {

            use torrust_tracker_configuration::Core;

            use crate::core_tests::sample_info_hash;
            use crate::whitelist::authorization::tests::the_whitelist_authorization_for_announce_and_scrape_actions::{
                initialize_whitelist_authorization_and_dependencies_with, initialize_whitelist_authorization_with,
            };

            fn configuration_for_non_listed_tracker() -> Core {
                Core {
                    listed: false,
                    ..Default::default()
                }
            }

            #[tokio::test]
            async fn should_authorize_a_whitelisted_infohash() {
                let (whitelist_authorization, in_memory_whitelist) =
                    initialize_whitelist_authorization_and_dependencies_with(&configuration_for_non_listed_tracker());

                let info_hash = sample_info_hash();

                let _unused = in_memory_whitelist.add(&info_hash).await;

                let result = whitelist_authorization.authorize(&info_hash).await;

                assert!(result.is_ok());
            }

            #[tokio::test]
            async fn should_also_authorize_a_non_whitelisted_infohash() {
                let whitelist_authorization = initialize_whitelist_authorization_with(&configuration_for_non_listed_tracker());

                let result = whitelist_authorization.authorize(&sample_info_hash()).await;

                assert!(result.is_ok());
            }
        }
    }
}
