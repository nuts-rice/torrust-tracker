//! The `scrape` service.
//!
//! The service is responsible for handling the `scrape` requests.
//!
//! It delegates the `scrape` logic to the [`ScrapeHandler`] and it returns the
//! [`ScrapeData`].
//!
//! It also sends an [`http_tracker_core::statistics::event::Event`]
//! because events are specific for the HTTP tracker.
use std::net::IpAddr;
use std::sync::Arc;

use bittorrent_primitives::info_hash::InfoHash;
use bittorrent_tracker_core::scrape_handler::ScrapeHandler;
use torrust_tracker_primitives::core::ScrapeData;

use crate::packages::http_tracker_core;

/// The HTTP tracker `scrape` service.
///
/// The service sends an statistics event that increments:
///
/// - The number of TCP connections handled by the HTTP tracker.
/// - The number of TCP `scrape` requests handled by the HTTP tracker.
///
/// > **NOTICE**: as the HTTP tracker does not requires a connection request
/// > like the UDP tracker, the number of TCP connections is incremented for
/// > each `scrape` request.
pub async fn invoke(
    scrape_handler: &Arc<ScrapeHandler>,
    opt_http_stats_event_sender: &Arc<Option<Box<dyn http_tracker_core::statistics::event::sender::Sender>>>,
    info_hashes: &Vec<InfoHash>,
    original_peer_ip: &IpAddr,
) -> ScrapeData {
    let scrape_data = scrape_handler.scrape(info_hashes).await;

    send_scrape_event(original_peer_ip, opt_http_stats_event_sender).await;

    scrape_data
}

/// The HTTP tracker fake `scrape` service. It returns zeroed stats.
///
/// When the peer is not authenticated and the tracker is running in `private` mode,
/// the tracker returns empty stats for all the torrents.
///
/// > **NOTICE**: tracker statistics are not updated in this case.
pub async fn fake(
    opt_http_stats_event_sender: &Arc<Option<Box<dyn http_tracker_core::statistics::event::sender::Sender>>>,
    info_hashes: &Vec<InfoHash>,
    original_peer_ip: &IpAddr,
) -> ScrapeData {
    send_scrape_event(original_peer_ip, opt_http_stats_event_sender).await;

    ScrapeData::zeroed(info_hashes)
}

async fn send_scrape_event(
    original_peer_ip: &IpAddr,
    opt_http_stats_event_sender: &Arc<Option<Box<dyn http_tracker_core::statistics::event::sender::Sender>>>,
) {
    if let Some(http_stats_event_sender) = opt_http_stats_event_sender.as_deref() {
        match original_peer_ip {
            IpAddr::V4(_) => {
                http_stats_event_sender
                    .send_event(http_tracker_core::statistics::event::Event::Tcp4Scrape)
                    .await;
            }
            IpAddr::V6(_) => {
                http_stats_event_sender
                    .send_event(http_tracker_core::statistics::event::Event::Tcp6Scrape)
                    .await;
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::sync::Arc;

    use aquatic_udp_protocol::{AnnounceEvent, NumberOfBytes, PeerId};
    use bittorrent_primitives::info_hash::InfoHash;
    use bittorrent_tracker_core::announce_handler::AnnounceHandler;
    use bittorrent_tracker_core::core_tests::sample_info_hash;
    use bittorrent_tracker_core::databases::setup::initialize_database;
    use bittorrent_tracker_core::scrape_handler::ScrapeHandler;
    use bittorrent_tracker_core::torrent::repository::in_memory::InMemoryTorrentRepository;
    use bittorrent_tracker_core::torrent::repository::persisted::DatabasePersistentTorrentRepository;
    use bittorrent_tracker_core::whitelist::authorization::WhitelistAuthorization;
    use bittorrent_tracker_core::whitelist::repository::in_memory::InMemoryWhitelist;
    use futures::future::BoxFuture;
    use mockall::mock;
    use tokio::sync::mpsc::error::SendError;
    use torrust_tracker_primitives::{peer, DurationSinceUnixEpoch};
    use torrust_tracker_test_helpers::configuration;

    use crate::packages::http_tracker_core;

    fn initialize_announce_and_scrape_handlers_for_public_tracker() -> (Arc<AnnounceHandler>, Arc<ScrapeHandler>) {
        let config = configuration::ephemeral_public();

        let database = initialize_database(&config.core);
        let in_memory_whitelist = Arc::new(InMemoryWhitelist::default());
        let whitelist_authorization = Arc::new(WhitelistAuthorization::new(&config.core, &in_memory_whitelist.clone()));
        let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());
        let db_torrent_repository = Arc::new(DatabasePersistentTorrentRepository::new(&database));
        let announce_handler = Arc::new(AnnounceHandler::new(
            &config.core,
            &in_memory_torrent_repository,
            &db_torrent_repository,
        ));
        let scrape_handler = Arc::new(ScrapeHandler::new(&whitelist_authorization, &in_memory_torrent_repository));

        (announce_handler, scrape_handler)
    }

    fn sample_info_hashes() -> Vec<InfoHash> {
        vec![sample_info_hash()]
    }

    fn sample_peer() -> peer::Peer {
        peer::Peer {
            peer_id: PeerId(*b"-qB00000000000000000"),
            peer_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(126, 0, 0, 1)), 8080),
            updated: DurationSinceUnixEpoch::new(1_669_397_478_934, 0),
            uploaded: NumberOfBytes::new(0),
            downloaded: NumberOfBytes::new(0),
            left: NumberOfBytes::new(0),
            event: AnnounceEvent::Started,
        }
    }

    fn initialize_scrape_handler() -> Arc<ScrapeHandler> {
        let config = configuration::ephemeral();

        let in_memory_whitelist = Arc::new(InMemoryWhitelist::default());
        let whitelist_authorization = Arc::new(WhitelistAuthorization::new(&config.core, &in_memory_whitelist.clone()));
        let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

        Arc::new(ScrapeHandler::new(&whitelist_authorization, &in_memory_torrent_repository))
    }

    mock! {
        HttpStatsEventSender {}
        impl http_tracker_core::statistics::event::sender::Sender for HttpStatsEventSender {
             fn send_event(&self, event: http_tracker_core::statistics::event::Event) -> BoxFuture<'static,Option<Result<(),SendError<http_tracker_core::statistics::event::Event> > > > ;
        }
    }

    mod with_real_data {

        use std::future;
        use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
        use std::sync::Arc;

        use bittorrent_tracker_core::announce_handler::PeersWanted;
        use mockall::predicate::eq;
        use torrust_tracker_primitives::core::ScrapeData;
        use torrust_tracker_primitives::swarm_metadata::SwarmMetadata;

        use crate::packages::{self, http_tracker_core};
        use crate::servers::http::v1::services::scrape::invoke;
        use crate::servers::http::v1::services::scrape::tests::{
            initialize_announce_and_scrape_handlers_for_public_tracker, initialize_scrape_handler, sample_info_hash,
            sample_info_hashes, sample_peer, MockHttpStatsEventSender,
        };

        #[tokio::test]
        async fn it_should_return_the_scrape_data_for_a_torrent() {
            let (http_stats_event_sender, _http_stats_repository) =
                packages::http_tracker_core::statistics::setup::factory(false);
            let http_stats_event_sender = Arc::new(http_stats_event_sender);

            let (announce_handler, scrape_handler) = initialize_announce_and_scrape_handlers_for_public_tracker();

            let info_hash = sample_info_hash();
            let info_hashes = vec![info_hash];

            // Announce a new peer to force scrape data to contain not zeroed data
            let mut peer = sample_peer();
            let original_peer_ip = peer.ip();
            announce_handler.announce(&info_hash, &mut peer, &original_peer_ip, &PeersWanted::All);

            let scrape_data = invoke(&scrape_handler, &http_stats_event_sender, &info_hashes, &original_peer_ip).await;

            let mut expected_scrape_data = ScrapeData::empty();
            expected_scrape_data.add_file(
                &info_hash,
                SwarmMetadata {
                    complete: 1,
                    downloaded: 0,
                    incomplete: 0,
                },
            );

            assert_eq!(scrape_data, expected_scrape_data);
        }

        #[tokio::test]
        async fn it_should_send_the_tcp_4_scrape_event_when_the_peer_uses_ipv4() {
            let mut http_stats_event_sender_mock = MockHttpStatsEventSender::new();
            http_stats_event_sender_mock
                .expect_send_event()
                .with(eq(http_tracker_core::statistics::event::Event::Tcp4Scrape))
                .times(1)
                .returning(|_| Box::pin(future::ready(Some(Ok(())))));
            let http_stats_event_sender: Arc<Option<Box<dyn http_tracker_core::statistics::event::sender::Sender>>> =
                Arc::new(Some(Box::new(http_stats_event_sender_mock)));

            let scrape_handler = initialize_scrape_handler();

            let peer_ip = IpAddr::V4(Ipv4Addr::new(126, 0, 0, 1));

            invoke(&scrape_handler, &http_stats_event_sender, &sample_info_hashes(), &peer_ip).await;
        }

        #[tokio::test]
        async fn it_should_send_the_tcp_6_scrape_event_when_the_peer_uses_ipv6() {
            let mut http_stats_event_sender_mock = MockHttpStatsEventSender::new();
            http_stats_event_sender_mock
                .expect_send_event()
                .with(eq(http_tracker_core::statistics::event::Event::Tcp6Scrape))
                .times(1)
                .returning(|_| Box::pin(future::ready(Some(Ok(())))));
            let http_stats_event_sender: Arc<Option<Box<dyn http_tracker_core::statistics::event::sender::Sender>>> =
                Arc::new(Some(Box::new(http_stats_event_sender_mock)));

            let scrape_handler = initialize_scrape_handler();

            let peer_ip = IpAddr::V6(Ipv6Addr::new(0x6969, 0x6969, 0x6969, 0x6969, 0x6969, 0x6969, 0x6969, 0x6969));

            invoke(&scrape_handler, &http_stats_event_sender, &sample_info_hashes(), &peer_ip).await;
        }
    }

    mod with_zeroed_data {

        use std::future;
        use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
        use std::sync::Arc;

        use bittorrent_tracker_core::announce_handler::PeersWanted;
        use mockall::predicate::eq;
        use torrust_tracker_primitives::core::ScrapeData;

        use crate::packages::{self, http_tracker_core};
        use crate::servers::http::v1::services::scrape::fake;
        use crate::servers::http::v1::services::scrape::tests::{
            initialize_announce_and_scrape_handlers_for_public_tracker, sample_info_hash, sample_info_hashes, sample_peer,
            MockHttpStatsEventSender,
        };

        #[tokio::test]
        async fn it_should_always_return_the_zeroed_scrape_data_for_a_torrent() {
            let (http_stats_event_sender, _http_stats_repository) =
                packages::http_tracker_core::statistics::setup::factory(false);
            let http_stats_event_sender = Arc::new(http_stats_event_sender);

            let (announce_handler, _scrape_handler) = initialize_announce_and_scrape_handlers_for_public_tracker();

            let info_hash = sample_info_hash();
            let info_hashes = vec![info_hash];

            // Announce a new peer to force scrape data to contain not zeroed data
            let mut peer = sample_peer();
            let original_peer_ip = peer.ip();
            announce_handler.announce(&info_hash, &mut peer, &original_peer_ip, &PeersWanted::All);

            let scrape_data = fake(&http_stats_event_sender, &info_hashes, &original_peer_ip).await;

            let expected_scrape_data = ScrapeData::zeroed(&info_hashes);

            assert_eq!(scrape_data, expected_scrape_data);
        }

        #[tokio::test]
        async fn it_should_send_the_tcp_4_scrape_event_when_the_peer_uses_ipv4() {
            let mut http_stats_event_sender_mock = MockHttpStatsEventSender::new();
            http_stats_event_sender_mock
                .expect_send_event()
                .with(eq(http_tracker_core::statistics::event::Event::Tcp4Scrape))
                .times(1)
                .returning(|_| Box::pin(future::ready(Some(Ok(())))));
            let http_stats_event_sender: Arc<Option<Box<dyn http_tracker_core::statistics::event::sender::Sender>>> =
                Arc::new(Some(Box::new(http_stats_event_sender_mock)));

            let peer_ip = IpAddr::V4(Ipv4Addr::new(126, 0, 0, 1));

            fake(&http_stats_event_sender, &sample_info_hashes(), &peer_ip).await;
        }

        #[tokio::test]
        async fn it_should_send_the_tcp_6_scrape_event_when_the_peer_uses_ipv6() {
            let mut http_stats_event_sender_mock = MockHttpStatsEventSender::new();
            http_stats_event_sender_mock
                .expect_send_event()
                .with(eq(http_tracker_core::statistics::event::Event::Tcp6Scrape))
                .times(1)
                .returning(|_| Box::pin(future::ready(Some(Ok(())))));
            let http_stats_event_sender: Arc<Option<Box<dyn http_tracker_core::statistics::event::sender::Sender>>> =
                Arc::new(Some(Box::new(http_stats_event_sender_mock)));

            let peer_ip = IpAddr::V6(Ipv6Addr::new(0x6969, 0x6969, 0x6969, 0x6969, 0x6969, 0x6969, 0x6969, 0x6969));

            fake(&http_stats_event_sender, &sample_info_hashes(), &peer_ip).await;
        }
    }
}
