use std::cmp::max;
use std::sync::Arc;

use bittorrent_primitives::info_hash::InfoHash;
use torrust_tracker_configuration::{TrackerPolicy, TORRENT_PEERS_LIMIT};
use torrust_tracker_primitives::pagination::Pagination;
use torrust_tracker_primitives::swarm_metadata::SwarmMetadata;
use torrust_tracker_primitives::torrent_metrics::TorrentsMetrics;
use torrust_tracker_primitives::{peer, DurationSinceUnixEpoch, PersistentTorrents};
use torrust_tracker_torrent_repository::entry::EntrySync;
use torrust_tracker_torrent_repository::repository::Repository;
use torrust_tracker_torrent_repository::EntryMutexStd;

use crate::torrent::Torrents;

/// The in-memory torrents repository.
///
/// There are many implementations of the repository trait. We tried with
/// different types of data structures, but the best performance was with
/// the one we use for production. We kept the other implementations for
/// reference.
#[derive(Debug, Default)]
pub struct InMemoryTorrentRepository {
    /// The in-memory torrents repository implementation.
    torrents: Arc<Torrents>,
}

impl InMemoryTorrentRepository {
    /// It inserts (or updates if it's already in the list) the peer in the
    /// torrent entry.
    pub fn upsert_peer(&self, info_hash: &InfoHash, peer: &peer::Peer) {
        self.torrents.upsert_peer(info_hash, peer);
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn remove(&self, key: &InfoHash) -> Option<EntryMutexStd> {
        self.torrents.remove(key)
    }

    pub(crate) fn remove_inactive_peers(&self, current_cutoff: DurationSinceUnixEpoch) {
        self.torrents.remove_inactive_peers(current_cutoff);
    }

    pub(crate) fn remove_peerless_torrents(&self, policy: &TrackerPolicy) {
        self.torrents.remove_peerless_torrents(policy);
    }

    #[must_use]
    pub(crate) fn get(&self, key: &InfoHash) -> Option<EntryMutexStd> {
        self.torrents.get(key)
    }

    #[must_use]
    pub(crate) fn get_paginated(&self, pagination: Option<&Pagination>) -> Vec<(InfoHash, EntryMutexStd)> {
        self.torrents.get_paginated(pagination)
    }

    /// It returns the data for a `scrape` response or empty if the torrent is
    /// not found.
    #[must_use]
    pub(crate) fn get_swarm_metadata(&self, info_hash: &InfoHash) -> SwarmMetadata {
        match self.torrents.get(info_hash) {
            Some(torrent_entry) => torrent_entry.get_swarm_metadata(),
            None => SwarmMetadata::zeroed(),
        }
    }

    /// Get torrent peers for a given torrent and client.
    ///
    /// It filters out the client making the request.
    #[must_use]
    pub(crate) fn get_peers_for(&self, info_hash: &InfoHash, peer: &peer::Peer, limit: usize) -> Vec<Arc<peer::Peer>> {
        match self.torrents.get(info_hash) {
            None => vec![],
            Some(entry) => entry.get_peers_for_client(&peer.peer_addr, Some(max(limit, TORRENT_PEERS_LIMIT))),
        }
    }

    /// Get torrent peers for a given torrent.
    #[must_use]
    pub fn get_torrent_peers(&self, info_hash: &InfoHash) -> Vec<Arc<peer::Peer>> {
        match self.torrents.get(info_hash) {
            None => vec![],
            Some(entry) => entry.get_peers(Some(TORRENT_PEERS_LIMIT)),
        }
    }

    /// It calculates and returns the general [`TorrentsMetrics`].
    #[must_use]
    pub fn get_torrents_metrics(&self) -> TorrentsMetrics {
        self.torrents.get_metrics()
    }

    pub fn import_persistent(&self, persistent_torrents: &PersistentTorrents) {
        self.torrents.import_persistent(persistent_torrents);
    }
}

#[cfg(test)]
mod tests {

    mod the_in_memory_torrent_repository {

        use aquatic_udp_protocol::PeerId;

        /// It generates a peer id from a number where the number is the last
        /// part of the peer ID. For example, for `12` it returns
        /// `-qB00000000000000012`.
        fn numeric_peer_id(two_digits_value: i32) -> PeerId {
            // Format idx as a string with leading zeros, ensuring it has exactly 2 digits
            let idx_str = format!("{two_digits_value:02}");

            // Create the base part of the peer ID.
            let base = b"-qB00000000000000000";

            // Concatenate the base with idx bytes, ensuring the total length is 20 bytes.
            let mut peer_id_bytes = [0u8; 20];
            peer_id_bytes[..base.len()].copy_from_slice(base);
            peer_id_bytes[base.len() - idx_str.len()..].copy_from_slice(idx_str.as_bytes());

            PeerId(peer_id_bytes)
        }

        // The `InMemoryTorrentRepository` has these responsibilities:
        // - To maintain the peer lists for each torrent.
        // - To maintain the the torrent entries, which contains all the info about the
        //   torrents, including the peer lists.
        // - To return the torrent entries.
        // - To return the peer lists for a given torrent.
        // - To return the torrent metrics.
        // - To return the swarm metadata for a given torrent.
        // - To handle the persistence of the torrent entries.

        mod maintaining_the_peer_lists {

            use std::sync::Arc;

            use crate::test_helpers::tests::{sample_info_hash, sample_peer};
            use crate::torrent::repository::in_memory::InMemoryTorrentRepository;

            #[tokio::test]
            async fn it_should_add_the_first_peer_to_the_torrent_peer_list() {
                let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

                let info_hash = sample_info_hash();

                let () = in_memory_torrent_repository.upsert_peer(&info_hash, &sample_peer());

                assert!(in_memory_torrent_repository.get(&info_hash).is_some());
            }

            #[tokio::test]
            async fn it_should_allow_adding_the_same_peer_twice_to_the_torrent_peer_list() {
                let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

                let info_hash = sample_info_hash();

                let () = in_memory_torrent_repository.upsert_peer(&info_hash, &sample_peer());
                let () = in_memory_torrent_repository.upsert_peer(&info_hash, &sample_peer());

                assert!(in_memory_torrent_repository.get(&info_hash).is_some());
            }
        }

        mod returning_peer_lists_for_a_torrent {

            use std::net::{IpAddr, Ipv4Addr, SocketAddr};
            use std::sync::Arc;

            use aquatic_udp_protocol::{AnnounceEvent, NumberOfBytes};
            use torrust_tracker_primitives::peer::Peer;
            use torrust_tracker_primitives::DurationSinceUnixEpoch;

            use crate::test_helpers::tests::{sample_info_hash, sample_peer};
            use crate::torrent::repository::in_memory::tests::the_in_memory_torrent_repository::numeric_peer_id;
            use crate::torrent::repository::in_memory::InMemoryTorrentRepository;

            #[tokio::test]
            async fn it_should_return_the_peers_for_a_given_torrent() {
                let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

                let info_hash = sample_info_hash();
                let peer = sample_peer();

                let () = in_memory_torrent_repository.upsert_peer(&info_hash, &peer);

                let peers = in_memory_torrent_repository.get_torrent_peers(&info_hash);

                assert_eq!(peers, vec![Arc::new(peer)]);
            }

            #[tokio::test]
            async fn it_should_return_an_empty_list_or_peers_for_a_non_existing_torrent() {
                let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

                let peers = in_memory_torrent_repository.get_torrent_peers(&sample_info_hash());

                assert!(peers.is_empty());
            }

            #[tokio::test]
            async fn it_should_return_74_peers_at_the_most_for_a_given_torrent() {
                let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

                let info_hash = sample_info_hash();

                for idx in 1..=75 {
                    let peer = Peer {
                        peer_id: numeric_peer_id(idx),
                        peer_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(126, 0, 0, idx.try_into().unwrap())), 8080),
                        updated: DurationSinceUnixEpoch::new(1_669_397_478_934, 0),
                        uploaded: NumberOfBytes::new(0),
                        downloaded: NumberOfBytes::new(0),
                        left: NumberOfBytes::new(0), // No bytes left to download
                        event: AnnounceEvent::Completed,
                    };

                    let () = in_memory_torrent_repository.upsert_peer(&info_hash, &peer);
                }

                let peers = in_memory_torrent_repository.get_torrent_peers(&info_hash);

                assert_eq!(peers.len(), 74);
            }

            mod excluding_the_client_peer {

                use std::net::{IpAddr, Ipv4Addr, SocketAddr};
                use std::sync::Arc;

                use aquatic_udp_protocol::{AnnounceEvent, NumberOfBytes};
                use torrust_tracker_configuration::TORRENT_PEERS_LIMIT;
                use torrust_tracker_primitives::peer::Peer;
                use torrust_tracker_primitives::DurationSinceUnixEpoch;

                use crate::test_helpers::tests::{sample_info_hash, sample_peer};
                use crate::torrent::repository::in_memory::tests::the_in_memory_torrent_repository::numeric_peer_id;
                use crate::torrent::repository::in_memory::InMemoryTorrentRepository;

                #[tokio::test]
                async fn it_should_return_an_empty_peer_list_for_a_non_existing_torrent() {
                    let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

                    let peers =
                        in_memory_torrent_repository.get_peers_for(&sample_info_hash(), &sample_peer(), TORRENT_PEERS_LIMIT);

                    assert_eq!(peers, vec![]);
                }

                #[tokio::test]
                async fn it_should_return_the_peers_for_a_given_torrent_excluding_a_given_peer() {
                    let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

                    let info_hash = sample_info_hash();
                    let peer = sample_peer();

                    let () = in_memory_torrent_repository.upsert_peer(&info_hash, &peer);

                    let peers = in_memory_torrent_repository.get_peers_for(&info_hash, &peer, TORRENT_PEERS_LIMIT);

                    assert_eq!(peers, vec![]);
                }

                #[tokio::test]
                async fn it_should_return_74_peers_at_the_most_for_a_given_torrent_when_it_filters_out_a_given_peer() {
                    let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

                    let info_hash = sample_info_hash();

                    let excluded_peer = sample_peer();

                    let () = in_memory_torrent_repository.upsert_peer(&info_hash, &excluded_peer);

                    // Add 74 peers
                    for idx in 2..=75 {
                        let peer = Peer {
                            peer_id: numeric_peer_id(idx),
                            peer_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(126, 0, 0, idx.try_into().unwrap())), 8080),
                            updated: DurationSinceUnixEpoch::new(1_669_397_478_934, 0),
                            uploaded: NumberOfBytes::new(0),
                            downloaded: NumberOfBytes::new(0),
                            left: NumberOfBytes::new(0), // No bytes left to download
                            event: AnnounceEvent::Completed,
                        };

                        let () = in_memory_torrent_repository.upsert_peer(&info_hash, &peer);
                    }

                    let peers = in_memory_torrent_repository.get_peers_for(&info_hash, &excluded_peer, TORRENT_PEERS_LIMIT);

                    assert_eq!(peers.len(), 74);
                }
            }
        }

        mod maintaining_the_torrent_entries {

            use std::ops::Add;
            use std::sync::Arc;
            use std::time::Duration;

            use bittorrent_primitives::info_hash::InfoHash;
            use torrust_tracker_configuration::TrackerPolicy;
            use torrust_tracker_primitives::DurationSinceUnixEpoch;

            use crate::test_helpers::tests::{sample_info_hash, sample_peer};
            use crate::torrent::repository::in_memory::InMemoryTorrentRepository;

            #[tokio::test]
            async fn it_should_remove_a_torrent_entry() {
                let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

                let info_hash = sample_info_hash();
                let () = in_memory_torrent_repository.upsert_peer(&info_hash, &sample_peer());

                let _unused = in_memory_torrent_repository.remove(&info_hash);

                assert!(in_memory_torrent_repository.get(&info_hash).is_none());
            }

            #[tokio::test]
            async fn it_should_remove_peers_that_have_not_been_updated_after_a_cutoff_time() {
                let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

                let info_hash = sample_info_hash();
                let mut peer = sample_peer();
                peer.updated = DurationSinceUnixEpoch::new(0, 0);

                let () = in_memory_torrent_repository.upsert_peer(&info_hash, &peer);

                // Cut off time is 1 second after the peer was updated
                in_memory_torrent_repository.remove_inactive_peers(peer.updated.add(Duration::from_secs(1)));

                assert!(!in_memory_torrent_repository
                    .get_torrent_peers(&info_hash)
                    .contains(&Arc::new(peer)));
            }

            fn initialize_repository_with_one_torrent_without_peers(info_hash: &InfoHash) -> Arc<InMemoryTorrentRepository> {
                let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

                // Insert a sample peer for the torrent to force adding the torrent entry
                let mut peer = sample_peer();
                peer.updated = DurationSinceUnixEpoch::new(0, 0);
                let () = in_memory_torrent_repository.upsert_peer(info_hash, &peer);

                // Remove the peer
                in_memory_torrent_repository.remove_inactive_peers(peer.updated.add(Duration::from_secs(1)));

                in_memory_torrent_repository
            }

            #[tokio::test]
            async fn it_should_remove_torrents_without_peers() {
                let info_hash = sample_info_hash();

                let in_memory_torrent_repository = initialize_repository_with_one_torrent_without_peers(&info_hash);

                let tracker_policy = TrackerPolicy {
                    remove_peerless_torrents: true,
                    ..Default::default()
                };

                in_memory_torrent_repository.remove_peerless_torrents(&tracker_policy);

                assert!(in_memory_torrent_repository.get(&info_hash).is_none());
            }
        }
        mod returning_torrent_entries {

            use std::sync::Arc;

            use torrust_tracker_primitives::peer::Peer;
            use torrust_tracker_primitives::swarm_metadata::SwarmMetadata;
            use torrust_tracker_torrent_repository::entry::EntrySync;

            use crate::test_helpers::tests::{sample_info_hash, sample_peer};
            use crate::torrent::repository::in_memory::InMemoryTorrentRepository;
            use crate::torrent::TorrentEntry;

            /// `TorrentEntry` data is not directly accessible. It's only
            /// accessible through the trait methods. We need this temporary
            /// DTO to write simple and more readable assertions.
            #[derive(Debug, Clone, PartialEq)]
            struct TorrentEntryInfo {
                swarm_metadata: SwarmMetadata,
                peers: Vec<Peer>,
                number_of_peers: usize,
            }

            #[allow(clippy::from_over_into)]
            impl Into<TorrentEntryInfo> for TorrentEntry {
                fn into(self) -> TorrentEntryInfo {
                    TorrentEntryInfo {
                        swarm_metadata: self.get_swarm_metadata(),
                        peers: self.get_peers(None).iter().map(|peer| *peer.clone()).collect(),
                        number_of_peers: self.get_peers_len(),
                    }
                }
            }

            #[tokio::test]
            async fn it_should_return_one_torrent_entry_by_infohash() {
                let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

                let info_hash = sample_info_hash();
                let peer = sample_peer();

                let () = in_memory_torrent_repository.upsert_peer(&info_hash, &peer);

                let torrent_entry = in_memory_torrent_repository.get(&info_hash).unwrap();

                assert_eq!(
                    TorrentEntryInfo {
                        swarm_metadata: SwarmMetadata {
                            downloaded: 0,
                            complete: 1,
                            incomplete: 0
                        },
                        peers: vec!(peer),
                        number_of_peers: 1
                    },
                    torrent_entry.into()
                );
            }

            mod it_should_return_many_torrent_entries {
                use std::sync::Arc;

                use torrust_tracker_primitives::swarm_metadata::SwarmMetadata;

                use crate::test_helpers::tests::{sample_info_hash, sample_peer};
                use crate::torrent::repository::in_memory::tests::the_in_memory_torrent_repository::returning_torrent_entries::TorrentEntryInfo;
                use crate::torrent::repository::in_memory::InMemoryTorrentRepository;

                #[tokio::test]
                async fn without_pagination() {
                    let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

                    let info_hash = sample_info_hash();
                    let peer = sample_peer();
                    let () = in_memory_torrent_repository.upsert_peer(&info_hash, &peer);

                    let torrent_entries = in_memory_torrent_repository.get_paginated(None);

                    assert_eq!(torrent_entries.len(), 1);

                    let torrent_entry = torrent_entries.first().unwrap().1.clone();

                    assert_eq!(
                        TorrentEntryInfo {
                            swarm_metadata: SwarmMetadata {
                                downloaded: 0,
                                complete: 1,
                                incomplete: 0
                            },
                            peers: vec!(peer),
                            number_of_peers: 1
                        },
                        torrent_entry.into()
                    );
                }

                mod with_pagination {
                    use std::sync::Arc;

                    use torrust_tracker_primitives::pagination::Pagination;
                    use torrust_tracker_primitives::swarm_metadata::SwarmMetadata;

                    use crate::test_helpers::tests::{
                        sample_info_hash_alphabetically_ordered_after_sample_info_hash_one, sample_info_hash_one,
                        sample_peer_one, sample_peer_two,
                    };
                    use crate::torrent::repository::in_memory::tests::the_in_memory_torrent_repository::returning_torrent_entries::TorrentEntryInfo;
                    use crate::torrent::repository::in_memory::InMemoryTorrentRepository;

                    #[tokio::test]
                    async fn it_should_return_the_first_page() {
                        let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

                        // Insert one torrent entry
                        let info_hash_one = sample_info_hash_one();
                        let peer_one = sample_peer_one();
                        let () = in_memory_torrent_repository.upsert_peer(&info_hash_one, &peer_one);

                        // Insert another torrent entry
                        let info_hash_one = sample_info_hash_alphabetically_ordered_after_sample_info_hash_one();
                        let peer_two = sample_peer_two();
                        let () = in_memory_torrent_repository.upsert_peer(&info_hash_one, &peer_two);

                        // Get only the first page where page size is 1
                        let torrent_entries =
                            in_memory_torrent_repository.get_paginated(Some(&Pagination { offset: 0, limit: 1 }));

                        assert_eq!(torrent_entries.len(), 1);

                        let torrent_entry = torrent_entries.first().unwrap().1.clone();

                        assert_eq!(
                            TorrentEntryInfo {
                                swarm_metadata: SwarmMetadata {
                                    downloaded: 0,
                                    complete: 1,
                                    incomplete: 0
                                },
                                peers: vec!(peer_one),
                                number_of_peers: 1
                            },
                            torrent_entry.into()
                        );
                    }

                    #[tokio::test]
                    async fn it_should_return_the_second_page() {
                        let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

                        // Insert one torrent entry
                        let info_hash_one = sample_info_hash_one();
                        let peer_one = sample_peer_one();
                        let () = in_memory_torrent_repository.upsert_peer(&info_hash_one, &peer_one);

                        // Insert another torrent entry
                        let info_hash_one = sample_info_hash_alphabetically_ordered_after_sample_info_hash_one();
                        let peer_two = sample_peer_two();
                        let () = in_memory_torrent_repository.upsert_peer(&info_hash_one, &peer_two);

                        // Get only the first page where page size is 1
                        let torrent_entries =
                            in_memory_torrent_repository.get_paginated(Some(&Pagination { offset: 1, limit: 1 }));

                        assert_eq!(torrent_entries.len(), 1);

                        let torrent_entry = torrent_entries.first().unwrap().1.clone();

                        assert_eq!(
                            TorrentEntryInfo {
                                swarm_metadata: SwarmMetadata {
                                    downloaded: 0,
                                    complete: 1,
                                    incomplete: 0
                                },
                                peers: vec!(peer_two),
                                number_of_peers: 1
                            },
                            torrent_entry.into()
                        );
                    }

                    #[tokio::test]
                    async fn it_should_allow_changing_the_page_size() {
                        let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

                        // Insert one torrent entry
                        let info_hash_one = sample_info_hash_one();
                        let peer_one = sample_peer_one();
                        let () = in_memory_torrent_repository.upsert_peer(&info_hash_one, &peer_one);

                        // Insert another torrent entry
                        let info_hash_one = sample_info_hash_alphabetically_ordered_after_sample_info_hash_one();
                        let peer_two = sample_peer_two();
                        let () = in_memory_torrent_repository.upsert_peer(&info_hash_one, &peer_two);

                        // Get only the first page where page size is 1
                        let torrent_entries =
                            in_memory_torrent_repository.get_paginated(Some(&Pagination { offset: 1, limit: 1 }));

                        assert_eq!(torrent_entries.len(), 1);
                    }
                }
            }
        }

        mod returning_torrent_metrics {

            use std::sync::Arc;

            use bittorrent_primitives::info_hash::fixture::gen_seeded_infohash;
            use torrust_tracker_primitives::torrent_metrics::TorrentsMetrics;

            use crate::test_helpers::tests::{complete_peer, leecher, sample_info_hash, seeder};
            use crate::torrent::repository::in_memory::InMemoryTorrentRepository;

            // todo: refactor to use test parametrization

            #[tokio::test]
            async fn it_should_get_empty_torrent_metrics_when_there_are_no_torrents() {
                let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

                let torrents_metrics = in_memory_torrent_repository.get_torrents_metrics();

                assert_eq!(
                    torrents_metrics,
                    TorrentsMetrics {
                        complete: 0,
                        downloaded: 0,
                        incomplete: 0,
                        torrents: 0
                    }
                );
            }

            #[tokio::test]
            async fn it_should_return_the_torrent_metrics_when_there_is_a_leecher() {
                let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

                let () = in_memory_torrent_repository.upsert_peer(&sample_info_hash(), &leecher());

                let torrent_metrics = in_memory_torrent_repository.get_torrents_metrics();

                assert_eq!(
                    torrent_metrics,
                    TorrentsMetrics {
                        complete: 0,
                        downloaded: 0,
                        incomplete: 1,
                        torrents: 1,
                    }
                );
            }

            #[tokio::test]
            async fn it_should_return_the_torrent_metrics_when_there_is_a_seeder() {
                let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

                let () = in_memory_torrent_repository.upsert_peer(&sample_info_hash(), &seeder());

                let torrent_metrics = in_memory_torrent_repository.get_torrents_metrics();

                assert_eq!(
                    torrent_metrics,
                    TorrentsMetrics {
                        complete: 1,
                        downloaded: 0,
                        incomplete: 0,
                        torrents: 1,
                    }
                );
            }

            #[tokio::test]
            async fn it_should_return_the_torrent_metrics_when_there_is_a_completed_peer() {
                let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

                let () = in_memory_torrent_repository.upsert_peer(&sample_info_hash(), &complete_peer());

                let torrent_metrics = in_memory_torrent_repository.get_torrents_metrics();

                assert_eq!(
                    torrent_metrics,
                    TorrentsMetrics {
                        complete: 1,
                        downloaded: 0,
                        incomplete: 0,
                        torrents: 1,
                    }
                );
            }

            #[tokio::test]
            async fn it_should_return_the_torrent_metrics_when_there_are_multiple_torrents() {
                let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

                let start_time = std::time::Instant::now();
                for i in 0..1_000_000 {
                    let () = in_memory_torrent_repository.upsert_peer(&gen_seeded_infohash(&i), &leecher());
                }
                let result_a = start_time.elapsed();

                let start_time = std::time::Instant::now();
                let torrent_metrics = in_memory_torrent_repository.get_torrents_metrics();
                let result_b = start_time.elapsed();

                assert_eq!(
                    (torrent_metrics),
                    (TorrentsMetrics {
                        complete: 0,
                        downloaded: 0,
                        incomplete: 1_000_000,
                        torrents: 1_000_000,
                    }),
                    "{result_a:?} {result_b:?}"
                );
            }
        }

        mod returning_swarm_metadata {

            use std::sync::Arc;

            use torrust_tracker_primitives::swarm_metadata::SwarmMetadata;

            use crate::test_helpers::tests::{leecher, sample_info_hash};
            use crate::torrent::repository::in_memory::InMemoryTorrentRepository;

            #[tokio::test]
            async fn it_should_get_swarm_metadata_for_an_existing_torrent() {
                let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

                let infohash = sample_info_hash();

                let () = in_memory_torrent_repository.upsert_peer(&infohash, &leecher());

                let swarm_metadata = in_memory_torrent_repository.get_swarm_metadata(&infohash);

                assert_eq!(
                    swarm_metadata,
                    SwarmMetadata {
                        complete: 0,
                        downloaded: 0,
                        incomplete: 1,
                    }
                );
            }

            #[tokio::test]
            async fn it_should_return_zeroed_swarm_metadata_for_a_non_existing_torrent() {
                let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

                let swarm_metadata = in_memory_torrent_repository.get_swarm_metadata(&sample_info_hash());

                assert_eq!(swarm_metadata, SwarmMetadata::zeroed());
            }
        }

        mod handling_persistence {

            use std::sync::Arc;

            use torrust_tracker_primitives::PersistentTorrents;

            use crate::test_helpers::tests::sample_info_hash;
            use crate::torrent::repository::in_memory::InMemoryTorrentRepository;

            #[tokio::test]
            async fn it_should_allow_importing_persisted_torrent_entries() {
                let in_memory_torrent_repository = Arc::new(InMemoryTorrentRepository::default());

                let infohash = sample_info_hash();

                let mut persistent_torrents = PersistentTorrents::default();

                persistent_torrents.insert(infohash, 1);

                in_memory_torrent_repository.import_persistent(&persistent_torrents);

                let swarm_metadata = in_memory_torrent_repository.get_swarm_metadata(&infohash);

                // Only the number of downloads is persisted.
                assert_eq!(swarm_metadata.downloaded, 1);
            }
        }
    }
}
