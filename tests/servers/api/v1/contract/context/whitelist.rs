use std::str::FromStr;

use torrust_tracker::shared::bit_torrent::info_hash::InfoHash;
use torrust_tracker_test_helpers::configuration;

use crate::servers::api::connection_info::{connection_with_invalid_token, connection_with_no_token};
use crate::servers::api::force_database_error;
use crate::servers::api::test_environment::running_test_environment;
use crate::servers::api::v1::asserts::{
    assert_failed_to_reload_whitelist, assert_failed_to_remove_torrent_from_whitelist, assert_failed_to_whitelist_torrent,
    assert_invalid_infohash_param, assert_not_found, assert_ok, assert_token_not_valid, assert_unauthorized,
};
use crate::servers::api::v1::client::Client;
use crate::servers::api::v1::contract::fixtures::{
    invalid_infohashes_returning_bad_request, invalid_infohashes_returning_not_found,
};

#[tokio::test]
async fn should_allow_whitelisting_a_torrent() {
    let test_env = running_test_environment(configuration::ephemeral()).await;

    let info_hash = "9e0217d0fa71c87332cd8bf9dbeabcb2c2cf3c4d".to_owned();

    let response = Client::new(test_env.get_connection_info())
        .whitelist_a_torrent(&info_hash)
        .await;

    assert_ok(response).await;
    assert!(
        test_env
            .tracker
            .is_info_hash_whitelisted(&InfoHash::from_str(&info_hash).unwrap())
            .await
    );

    test_env.stop().await;
}

#[tokio::test]
async fn should_allow_whitelisting_a_torrent_that_has_been_already_whitelisted() {
    let test_env = running_test_environment(configuration::ephemeral()).await;

    let info_hash = "9e0217d0fa71c87332cd8bf9dbeabcb2c2cf3c4d".to_owned();

    let api_client = Client::new(test_env.get_connection_info());

    let response = api_client.whitelist_a_torrent(&info_hash).await;
    assert_ok(response).await;

    let response = api_client.whitelist_a_torrent(&info_hash).await;
    assert_ok(response).await;

    test_env.stop().await;
}

#[tokio::test]
async fn should_not_allow_whitelisting_a_torrent_for_unauthenticated_users() {
    let test_env = running_test_environment(configuration::ephemeral()).await;

    let info_hash = "9e0217d0fa71c87332cd8bf9dbeabcb2c2cf3c4d".to_owned();

    let response = Client::new(connection_with_invalid_token(
        test_env.get_connection_info().bind_address.as_str(),
    ))
    .whitelist_a_torrent(&info_hash)
    .await;

    assert_token_not_valid(response).await;

    let response = Client::new(connection_with_no_token(test_env.get_connection_info().bind_address.as_str()))
        .whitelist_a_torrent(&info_hash)
        .await;

    assert_unauthorized(response).await;

    test_env.stop().await;
}

#[tokio::test]
async fn should_fail_when_the_torrent_cannot_be_whitelisted() {
    let test_env = running_test_environment(configuration::ephemeral()).await;

    let info_hash = "9e0217d0fa71c87332cd8bf9dbeabcb2c2cf3c4d".to_owned();

    force_database_error(&test_env.tracker);

    let response = Client::new(test_env.get_connection_info())
        .whitelist_a_torrent(&info_hash)
        .await;

    assert_failed_to_whitelist_torrent(response).await;

    test_env.stop().await;
}

#[tokio::test]
async fn should_fail_whitelisting_a_torrent_when_the_provided_infohash_is_invalid() {
    let test_env = running_test_environment(configuration::ephemeral()).await;

    for invalid_infohash in &invalid_infohashes_returning_bad_request() {
        let response = Client::new(test_env.get_connection_info())
            .whitelist_a_torrent(invalid_infohash)
            .await;

        assert_invalid_infohash_param(response, invalid_infohash).await;
    }

    for invalid_infohash in &invalid_infohashes_returning_not_found() {
        let response = Client::new(test_env.get_connection_info())
            .whitelist_a_torrent(invalid_infohash)
            .await;

        assert_not_found(response).await;
    }

    test_env.stop().await;
}

#[tokio::test]
async fn should_allow_removing_a_torrent_from_the_whitelist() {
    let test_env = running_test_environment(configuration::ephemeral()).await;

    let hash = "9e0217d0fa71c87332cd8bf9dbeabcb2c2cf3c4d".to_owned();
    let info_hash = InfoHash::from_str(&hash).unwrap();
    test_env.tracker.add_torrent_to_whitelist(&info_hash).await.unwrap();

    let response = Client::new(test_env.get_connection_info())
        .remove_torrent_from_whitelist(&hash)
        .await;

    assert_ok(response).await;
    assert!(!test_env.tracker.is_info_hash_whitelisted(&info_hash).await);

    test_env.stop().await;
}

#[tokio::test]
async fn should_not_fail_trying_to_remove_a_non_whitelisted_torrent_from_the_whitelist() {
    let test_env = running_test_environment(configuration::ephemeral()).await;

    let non_whitelisted_torrent_hash = "9e0217d0fa71c87332cd8bf9dbeabcb2c2cf3c4d".to_owned();

    let response = Client::new(test_env.get_connection_info())
        .remove_torrent_from_whitelist(&non_whitelisted_torrent_hash)
        .await;

    assert_ok(response).await;

    test_env.stop().await;
}

#[tokio::test]
async fn should_fail_removing_a_torrent_from_the_whitelist_when_the_provided_infohash_is_invalid() {
    let test_env = running_test_environment(configuration::ephemeral()).await;

    for invalid_infohash in &invalid_infohashes_returning_bad_request() {
        let response = Client::new(test_env.get_connection_info())
            .remove_torrent_from_whitelist(invalid_infohash)
            .await;

        assert_invalid_infohash_param(response, invalid_infohash).await;
    }

    for invalid_infohash in &invalid_infohashes_returning_not_found() {
        let response = Client::new(test_env.get_connection_info())
            .remove_torrent_from_whitelist(invalid_infohash)
            .await;

        assert_not_found(response).await;
    }

    test_env.stop().await;
}

#[tokio::test]
async fn should_fail_when_the_torrent_cannot_be_removed_from_the_whitelist() {
    let test_env = running_test_environment(configuration::ephemeral()).await;

    let hash = "9e0217d0fa71c87332cd8bf9dbeabcb2c2cf3c4d".to_owned();
    let info_hash = InfoHash::from_str(&hash).unwrap();
    test_env.tracker.add_torrent_to_whitelist(&info_hash).await.unwrap();

    force_database_error(&test_env.tracker);

    let response = Client::new(test_env.get_connection_info())
        .remove_torrent_from_whitelist(&hash)
        .await;

    assert_failed_to_remove_torrent_from_whitelist(response).await;

    test_env.stop().await;
}

#[tokio::test]
async fn should_not_allow_removing_a_torrent_from_the_whitelist_for_unauthenticated_users() {
    let test_env = running_test_environment(configuration::ephemeral()).await;

    let hash = "9e0217d0fa71c87332cd8bf9dbeabcb2c2cf3c4d".to_owned();
    let info_hash = InfoHash::from_str(&hash).unwrap();

    test_env.tracker.add_torrent_to_whitelist(&info_hash).await.unwrap();
    let response = Client::new(connection_with_invalid_token(
        test_env.get_connection_info().bind_address.as_str(),
    ))
    .remove_torrent_from_whitelist(&hash)
    .await;

    assert_token_not_valid(response).await;

    test_env.tracker.add_torrent_to_whitelist(&info_hash).await.unwrap();
    let response = Client::new(connection_with_no_token(test_env.get_connection_info().bind_address.as_str()))
        .remove_torrent_from_whitelist(&hash)
        .await;

    assert_unauthorized(response).await;

    test_env.stop().await;
}

#[tokio::test]
async fn should_allow_reload_the_whitelist_from_the_database() {
    let test_env = running_test_environment(configuration::ephemeral()).await;

    let hash = "9e0217d0fa71c87332cd8bf9dbeabcb2c2cf3c4d".to_owned();
    let info_hash = InfoHash::from_str(&hash).unwrap();
    test_env.tracker.add_torrent_to_whitelist(&info_hash).await.unwrap();

    let response = Client::new(test_env.get_connection_info()).reload_whitelist().await;

    assert_ok(response).await;
    /* todo: this assert fails because the whitelist has not been reloaded yet.
       We could add a new endpoint GET /api/whitelist/:info_hash to check if a torrent
       is whitelisted and use that endpoint to check if the torrent is still there after reloading.
    assert!(
        !(test_env
            .tracker
            .is_info_hash_whitelisted(&InfoHash::from_str(&info_hash).unwrap())
            .await)
    );
    */

    test_env.stop().await;
}

#[tokio::test]
async fn should_fail_when_the_whitelist_cannot_be_reloaded_from_the_database() {
    let test_env = running_test_environment(configuration::ephemeral()).await;

    let hash = "9e0217d0fa71c87332cd8bf9dbeabcb2c2cf3c4d".to_owned();
    let info_hash = InfoHash::from_str(&hash).unwrap();
    test_env.tracker.add_torrent_to_whitelist(&info_hash).await.unwrap();

    force_database_error(&test_env.tracker);

    let response = Client::new(test_env.get_connection_info()).reload_whitelist().await;

    assert_failed_to_reload_whitelist(response).await;

    test_env.stop().await;
}
