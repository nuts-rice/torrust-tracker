#!/bin/bash

# This script is only intended to be used for local development or testing environments.

# Generate benches directory if it does not exist
mkdir -p ./benches

cargo bench --package torrust-tracker-torrent-repository 
cargo bench --package bittorrent-http-tracker-core 
mv -b target/criterion/http_tracker_handle_announce_once/ ./benches/http_tracker_handle_announce_once
cargo bench --package bittorrent-udp-tracker-core 
mv -b target/criterion/udp_tracker_connect_once/ ./benches/udp_tracker_connect_once

