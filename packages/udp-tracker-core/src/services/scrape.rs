//! The `scrape` service.
//!
//! The service is responsible for handling the `scrape` requests.
//!
//! It delegates the `scrape` logic to the [`ScrapeHandler`] and it returns the
//! [`ScrapeData`].
//!
//! It also sends an [`udp_tracker_core::statistics::event::Event`]
//! because events are specific for the UDP tracker.
use std::net::SocketAddr;
use std::ops::Range;
use std::sync::Arc;

use aquatic_udp_protocol::ScrapeRequest;
use bittorrent_primitives::info_hash::InfoHash;
use bittorrent_tracker_core::error::{ScrapeError, WhitelistError};
use bittorrent_tracker_core::scrape_handler::ScrapeHandler;
use torrust_tracker_primitives::core::ScrapeData;

use crate::connection_cookie::{check, gen_remote_fingerprint, ConnectionCookieError};
use crate::statistics;

/// The `ScrapeService` is responsible for handling the `scrape` requests.
///
/// The service sends an statistics event that increments:
///
/// - The number of UDP `scrape` requests handled by the UDP tracker.
pub struct ScrapeService {
    scrape_handler: Arc<ScrapeHandler>,
    opt_udp_stats_event_sender: Arc<Option<Box<dyn statistics::event::sender::Sender>>>,
}

impl ScrapeService {
    #[must_use]
    pub fn new(
        scrape_handler: Arc<ScrapeHandler>,
        opt_udp_stats_event_sender: Arc<Option<Box<dyn statistics::event::sender::Sender>>>,
    ) -> Self {
        Self {
            scrape_handler,
            opt_udp_stats_event_sender,
        }
    }

    /// It handles the `Scrape` request.
    ///
    /// # Errors
    ///
    /// It will return an error if the tracker core scrape handler returns an error.
    pub async fn handle_scrape(
        &self,
        remote_client_addr: SocketAddr,
        request: &ScrapeRequest,
        cookie_valid_range: Range<f64>,
    ) -> Result<ScrapeData, UdpScrapeError> {
        Self::authenticate(remote_client_addr, request, cookie_valid_range)?;

        let scrape_data = self
            .scrape_handler
            .scrape(&Self::convert_from_aquatic(&request.info_hashes))
            .await?;

        self.send_stats_event(remote_client_addr).await;

        Ok(scrape_data)
    }

    fn authenticate(
        remote_addr: SocketAddr,
        request: &ScrapeRequest,
        cookie_valid_range: Range<f64>,
    ) -> Result<f64, ConnectionCookieError> {
        check(
            &request.connection_id,
            gen_remote_fingerprint(&remote_addr),
            cookie_valid_range,
        )
    }

    fn convert_from_aquatic(aquatic_infohashes: &[aquatic_udp_protocol::common::InfoHash]) -> Vec<InfoHash> {
        aquatic_infohashes.iter().map(|&x| x.into()).collect()
    }

    async fn send_stats_event(&self, remote_addr: SocketAddr) {
        if let Some(udp_stats_event_sender) = self.opt_udp_stats_event_sender.as_deref() {
            let event = match remote_addr {
                SocketAddr::V4(_) => statistics::event::Event::Udp4Scrape,
                SocketAddr::V6(_) => statistics::event::Event::Udp6Scrape,
            };
            udp_stats_event_sender.send_event(event).await;
        }
    }
}

/// Errors related to scrape requests.
#[derive(thiserror::Error, Debug, Clone)]
pub enum UdpScrapeError {
    /// Error returned when there was an error with the connection cookie.
    #[error("Connection cookie error: {source}")]
    ConnectionCookieError { source: ConnectionCookieError },

    /// Error returned when there was an error with the tracker core scrape handler.
    #[error("Tracker core scrape error: {source}")]
    TrackerCoreScrapeError { source: ScrapeError },

    /// Error returned when there was an error with the tracker core whitelist.
    #[error("Tracker core whitelist error: {source}")]
    TrackerCoreWhitelistError { source: WhitelistError },
}

impl From<ConnectionCookieError> for UdpScrapeError {
    fn from(connection_cookie_error: ConnectionCookieError) -> Self {
        Self::ConnectionCookieError {
            source: connection_cookie_error,
        }
    }
}

impl From<ScrapeError> for UdpScrapeError {
    fn from(scrape_error: ScrapeError) -> Self {
        Self::TrackerCoreScrapeError { source: scrape_error }
    }
}

impl From<WhitelistError> for UdpScrapeError {
    fn from(whitelist_error: WhitelistError) -> Self {
        Self::TrackerCoreWhitelistError { source: whitelist_error }
    }
}

#[cfg(test)]
mod tests {
    use aquatic_udp_protocol::{ConnectionId, InfoHash, ScrapeRequest, TransactionId};
    use bittorrent_tracker_core::torrent::repository::in_memory::InMemoryTorrentRepository;
    use bittorrent_tracker_core::whitelist;
    use bittorrent_tracker_core::whitelist::repository::in_memory::InMemoryWhitelist;
    use torrust_tracker_test_helpers::configuration;

    use super::*;
    //use crate::connection_cookie::make;
    use crate::services::tests::{
        sample_cookie_valid_range,
        //   sample_ipv4_remote_addr_fingerprint,
        //   sample_issue_time
        sample_ipv4_remote_addr,
    };
    use crate::statistics::repository::Repository;

    #[tokio::test]
    async fn should_increase_scrape_event_when_handling_scrape() {
        let config = configuration::ephemeral_public();
        let _stats_repository = Repository::new();
        let in_memory_whitelist = Arc::new(InMemoryWhitelist::default());
        let whitelist_authorization = Arc::new(whitelist::authorization::WhitelistAuthorization::new(
            &config.core,
            &in_memory_whitelist.clone(),
        ));
        let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());
        let (udp_core_stats_event_sender, _udp_core_stats_repository) = statistics::setup::factory(false);
        let udp_core_stats_event_sender = Arc::new(udp_core_stats_event_sender);
        let info_hash = InfoHash([0u8; 20]);
        let info_hashes = vec![info_hash];
        let request = ScrapeRequest {
            connection_id: ConnectionId(0i64.into()),
            transaction_id: TransactionId(0i32.into()),
            info_hashes,
        };

        let scrape_handler = Arc::new(ScrapeHandler::new(&whitelist_authorization, &in_memory_torrent_repository));
        let scrape_service = ScrapeService::new(scrape_handler, udp_core_stats_event_sender);
        let _result = scrape_service
            .handle_scrape(sample_ipv4_remote_addr(), &request, sample_cookie_valid_range())
            .await;
        /*let _stats = stats_repository.get_stats().await;
                    assert_eq!(
                        response.unwrap().files.keys().next().unwrap().0,
                        info_hash.0,

                    );
        */
        //  );
        //assert_eq!(stats.udp4_scrapes_handled, 1)
    }
}
