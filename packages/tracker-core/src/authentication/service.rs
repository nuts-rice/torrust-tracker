use std::panic::Location;
use std::sync::Arc;

use torrust_tracker_configuration::Core;

use super::key::repository::in_memory::InMemoryKeyRepository;
use super::{key, Error, Key};

#[derive(Debug)]
pub struct AuthenticationService {
    /// The tracker configuration.
    config: Core,

    /// In-memory implementation of the authentication key repository.
    in_memory_key_repository: Arc<InMemoryKeyRepository>,
}

impl AuthenticationService {
    #[must_use]
    pub fn new(config: &Core, in_memory_key_repository: &Arc<InMemoryKeyRepository>) -> Self {
        Self {
            config: config.clone(),
            in_memory_key_repository: in_memory_key_repository.clone(),
        }
    }

    /// It authenticates the peer `key` against the `Tracker` authentication
    /// key list.
    ///
    /// # Errors
    ///
    /// Will return an error if the the authentication key cannot be verified.
    pub async fn authenticate(&self, key: &Key) -> Result<(), Error> {
        if self.tracker_is_private() {
            self.verify_auth_key(key).await
        } else {
            Ok(())
        }
    }

    /// Returns `true` is the tracker is in private mode.
    #[must_use]
    fn tracker_is_private(&self) -> bool {
        self.config.private
    }

    /// It verifies an authentication key.
    ///
    /// # Errors
    ///
    /// Will return a `key::Error` if unable to get any `auth_key`.
    async fn verify_auth_key(&self, key: &Key) -> Result<(), Error> {
        match self.in_memory_key_repository.get(key).await {
            None => Err(Error::UnableToReadKey {
                location: Location::caller(),
                key: Box::new(key.clone()),
            }),
            Some(key) => match self.config.private_mode {
                Some(private_mode) => {
                    if private_mode.check_keys_expiration {
                        return key::verify_key_expiration(&key);
                    }

                    Ok(())
                }
                None => key::verify_key_expiration(&key),
            },
        }
    }
}

#[cfg(test)]
mod tests {

    mod the_authentication_service {

        mod when_the_tracker_is_public {

            use std::str::FromStr;
            use std::sync::Arc;

            use torrust_tracker_configuration::Core;

            use crate::authentication::key::repository::in_memory::InMemoryKeyRepository;
            use crate::authentication::service::AuthenticationService;
            use crate::authentication::{self};

            fn instantiate_authentication_for_public_tracker() -> AuthenticationService {
                let config = Core {
                    private: false,
                    ..Default::default()
                };

                let in_memory_key_repository = Arc::new(InMemoryKeyRepository::default());

                AuthenticationService::new(&config, &in_memory_key_repository.clone())
            }

            #[tokio::test]
            async fn it_should_always_authenticate_when_the_tracker_is_public() {
                let authentication = instantiate_authentication_for_public_tracker();

                let unregistered_key = authentication::Key::from_str("YZSl4lMZupRuOpSRC3krIKR5BPB14nrJ").unwrap();

                let result = authentication.authenticate(&unregistered_key).await;

                assert!(result.is_ok());
            }
        }

        mod when_the_tracker_is_private {

            use std::str::FromStr;
            use std::sync::Arc;
            use std::time::Duration;

            use torrust_tracker_configuration::v2_0_0::core::PrivateMode;
            use torrust_tracker_configuration::Core;

            use crate::authentication::key::repository::in_memory::InMemoryKeyRepository;
            use crate::authentication::service::AuthenticationService;
            use crate::authentication::{self, PeerKey};

            fn instantiate_authentication_for_private_tracker() -> AuthenticationService {
                let config = Core {
                    private: true,
                    ..Default::default()
                };

                let in_memory_key_repository = Arc::new(InMemoryKeyRepository::default());

                AuthenticationService::new(&config, &in_memory_key_repository.clone())
            }

            #[tokio::test]
            async fn it_should_authenticate_a_registered_key() {
                let config = Core {
                    private: true,
                    ..Default::default()
                };

                let in_memory_key_repository = Arc::new(InMemoryKeyRepository::default());

                let key = authentication::Key::from_str("YZSl4lMZupRuOpSRC3krIKR5BPB14nrJ").unwrap();

                in_memory_key_repository
                    .insert(&PeerKey {
                        key: key.clone(),
                        valid_until: None,
                    })
                    .await;

                let authentication = AuthenticationService::new(&config, &in_memory_key_repository.clone());

                let result = authentication.authenticate(&key).await;

                assert!(result.is_ok());
            }

            #[tokio::test]
            async fn it_should_not_authenticate_an_unregistered_key() {
                let authentication = instantiate_authentication_for_private_tracker();

                let unregistered_key = authentication::Key::from_str("YZSl4lMZupRuOpSRC3krIKR5BPB14nrJ").unwrap();

                let result = authentication.authenticate(&unregistered_key).await;

                assert!(result.is_err());
            }

            #[tokio::test]
            async fn it_should_not_authenticate_a_registered_but_expired_key_by_default() {
                let config = Core {
                    private: true,
                    ..Default::default()
                };

                let in_memory_key_repository = Arc::new(InMemoryKeyRepository::default());

                let key = authentication::Key::from_str("YZSl4lMZupRuOpSRC3krIKR5BPB14nrJ").unwrap();

                // Register the key with an immediate expiration date.
                in_memory_key_repository
                    .insert(&PeerKey {
                        key: key.clone(),
                        valid_until: Some(Duration::from_secs(0)),
                    })
                    .await;

                let authentication = AuthenticationService::new(&config, &in_memory_key_repository.clone());

                let result = authentication.authenticate(&key).await;

                assert!(result.is_err());
            }

            #[tokio::test]
            async fn it_should_not_authenticate_a_registered_but_expired_key_when_the_tracker_is_explicitly_configured_to_check_keys_expiration(
            ) {
                let config = Core {
                    private: true,
                    private_mode: Some(PrivateMode {
                        check_keys_expiration: true,
                    }),
                    ..Default::default()
                };

                let in_memory_key_repository = Arc::new(InMemoryKeyRepository::default());

                let key = authentication::Key::from_str("YZSl4lMZupRuOpSRC3krIKR5BPB14nrJ").unwrap();

                // Register the key with an immediate expiration date.
                in_memory_key_repository
                    .insert(&PeerKey {
                        key: key.clone(),
                        valid_until: Some(Duration::from_secs(0)),
                    })
                    .await;

                let authentication = AuthenticationService::new(&config, &in_memory_key_repository.clone());

                let result = authentication.authenticate(&key).await;

                assert!(result.is_err());
            }

            mod but_the_key_expiration_check_is_disabled_by_configuration {
                use std::str::FromStr;
                use std::sync::Arc;
                use std::time::Duration;

                use torrust_tracker_configuration::v2_0_0::core::PrivateMode;
                use torrust_tracker_configuration::Core;

                use crate::authentication::key::repository::in_memory::InMemoryKeyRepository;
                use crate::authentication::service::AuthenticationService;
                use crate::authentication::{self, PeerKey};

                #[tokio::test]
                async fn it_should_authenticate_an_expired_registered_key() {
                    let config = Core {
                        private: true,
                        private_mode: Some(PrivateMode {
                            check_keys_expiration: false,
                        }),
                        ..Default::default()
                    };

                    let in_memory_key_repository = Arc::new(InMemoryKeyRepository::default());

                    let key = authentication::Key::from_str("YZSl4lMZupRuOpSRC3krIKR5BPB14nrJ").unwrap();

                    // Register the key with an immediate expiration date.
                    in_memory_key_repository
                        .insert(&PeerKey {
                            key: key.clone(),
                            valid_until: Some(Duration::from_secs(0)),
                        })
                        .await;

                    let authentication = AuthenticationService::new(&config, &in_memory_key_repository.clone());

                    let result = authentication.authenticate(&key).await;

                    assert!(result.is_ok());
                }
            }
        }
    }
}
