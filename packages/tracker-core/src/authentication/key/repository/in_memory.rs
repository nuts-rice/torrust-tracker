use crate::authentication::key::{Key, PeerKey};

/// In-memory implementation of the authentication key repository.
#[derive(Debug, Default)]
pub struct InMemoryKeyRepository {
    /// Tracker users' keys. Only for private trackers.
    keys: tokio::sync::RwLock<std::collections::HashMap<Key, PeerKey>>,
}

impl InMemoryKeyRepository {
    /// It adds a new authentication key.
    pub async fn insert(&self, auth_key: &PeerKey) {
        self.keys.write().await.insert(auth_key.key.clone(), auth_key.clone());
    }

    /// It removes an authentication key.
    pub async fn remove(&self, key: &Key) {
        self.keys.write().await.remove(key);
    }

    pub async fn get(&self, key: &Key) -> Option<PeerKey> {
        self.keys.read().await.get(key).cloned()
    }

    /// It clears all the authentication keys.
    pub async fn clear(&self) {
        let mut keys = self.keys.write().await;
        keys.clear();
    }

    /// It resets the authentication keys with a new list of keys.
    pub async fn reset_with(&self, peer_keys: Vec<PeerKey>) {
        let mut keys_lock = self.keys.write().await;

        keys_lock.clear();

        for key in peer_keys {
            keys_lock.insert(key.key.clone(), key.clone());
        }
    }
}

#[cfg(test)]
mod tests {

    mod the_in_memory_key_repository_should {
        use std::time::Duration;

        use crate::authentication::key::repository::in_memory::InMemoryKeyRepository;
        use crate::authentication::key::Key;
        use crate::authentication::PeerKey;

        #[tokio::test]
        async fn insert_a_new_peer_key() {
            let repository = InMemoryKeyRepository::default();

            let new_peer_key = PeerKey {
                key: Key::new("YZSl4lMZupRuOpSRC3krIKR5BPB14nrJ").unwrap(),
                valid_until: Some(Duration::new(9999, 0)),
            };

            repository.insert(&new_peer_key).await;

            let peer_key = repository.get(&new_peer_key.key).await;

            assert_eq!(peer_key, Some(new_peer_key));
        }

        #[tokio::test]
        async fn remove_a_new_peer_key() {
            let repository = InMemoryKeyRepository::default();

            let new_peer_key = PeerKey {
                key: Key::new("YZSl4lMZupRuOpSRC3krIKR5BPB14nrJ").unwrap(),
                valid_until: Some(Duration::new(9999, 0)),
            };

            repository.insert(&new_peer_key).await;

            repository.remove(&new_peer_key.key).await;

            let peer_key = repository.get(&new_peer_key.key).await;

            assert_eq!(peer_key, None);
        }

        #[tokio::test]
        async fn get_a_new_peer_key_by_its_internal_key() {
            let repository = InMemoryKeyRepository::default();

            let expected_peer_key = PeerKey {
                key: Key::new("YZSl4lMZupRuOpSRC3krIKR5BPB14nrJ").unwrap(),
                valid_until: Some(Duration::new(9999, 0)),
            };

            repository.insert(&expected_peer_key).await;

            let peer_key = repository.get(&expected_peer_key.key).await;

            assert_eq!(peer_key, Some(expected_peer_key));
        }

        #[tokio::test]
        async fn clear_all_peer_keys() {
            let repository = InMemoryKeyRepository::default();

            let new_peer_key = PeerKey {
                key: Key::new("YZSl4lMZupRuOpSRC3krIKR5BPB14nrJ").unwrap(),
                valid_until: Some(Duration::new(9999, 0)),
            };

            repository.insert(&new_peer_key).await;

            repository.clear().await;

            let peer_key = repository.get(&new_peer_key.key).await;

            assert_eq!(peer_key, None);
        }

        #[tokio::test]
        async fn reset_the_peer_keys_with_a_new_list_of_keys() {
            let repository = InMemoryKeyRepository::default();

            let old_peer_key = PeerKey {
                key: Key::new("YZSl4lMZupRuOpSRC3krIKR5BPB14nrJ").unwrap(),
                valid_until: Some(Duration::new(9999, 0)),
            };

            repository.insert(&old_peer_key).await;

            let new_peer_key = PeerKey {
                key: Key::new("kqdVKHlKKWXzAideqI5gvjBP4jdbe5dW").unwrap(),
                valid_until: Some(Duration::new(9999, 0)),
            };

            repository.reset_with(vec![new_peer_key.clone()]).await;

            let peer_key = repository.get(&new_peer_key.key).await;

            assert_eq!(peer_key, Some(new_peer_key));
        }
    }
}
