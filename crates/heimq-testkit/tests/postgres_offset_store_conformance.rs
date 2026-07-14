//! Conformance suite run against the Postgres-backed offset store.
//!
//! Requires `--features backend-postgres`. Spins up a Postgres container via
//! testcontainers — no external instance needed. Skipped unless the
//! `HEIMQ_PG_TESTS` or `PARITY_TESTS` env var is set to avoid pulling Docker
//! images in CI jobs that don't have Docker available.
//!
//! Note: `PostgresOffsetStore::connect` uses the blocking `postgres` crate,
//! which internally calls `block_on`. All store operations must run on a
//! `spawn_blocking` thread to avoid nesting runtimes inside `#[tokio::test]`.

#![cfg(feature = "backend-postgres")]

use heimq::storage::PostgresOffsetStore;
use heimq_testkit::suites;
use testcontainers::core::WaitFor;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, GenericImage, ImageExt};

fn pg_tests_enabled() -> bool {
    std::env::var("HEIMQ_PG_TESTS").is_ok() || std::env::var("PARITY_TESTS").is_ok()
}

// with_wait_for is a native GenericImage method (must precede ImageExt methods
// which return ContainerRequest<GenericImage> and don't have with_wait_for).
async fn start_postgres() -> (ContainerAsync<GenericImage>, String) {
    let container: ContainerAsync<GenericImage> = GenericImage::new("postgres", "16")
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_PASSWORD", "testpass")
        .with_env_var("POSTGRES_USER", "heimq")
        .with_env_var("POSTGRES_DB", "heimq_test")
        .start()
        .await
        .expect("failed to start postgres container");

    let bridge_ip = container
        .get_bridge_ip_address()
        .await
        .expect("failed to get bridge IP");

    let url = format!("postgres://heimq:testpass@{}:5432/heimq_test", bridge_ip);

    (container, url)
}

#[tokio::test]
// @covers US-016-AC1
async fn postgres_offset_store_capabilities() {
    if !pg_tests_enabled() {
        return;
    }
    let (_container, url) = start_postgres().await;
    tokio::task::spawn_blocking(move || {
        let store = PostgresOffsetStore::connect(&url).expect("connect");
        suites::offset_store::check_capabilities(store.as_ref());
    })
    .await
    .expect("task panicked");
}

#[tokio::test]
// @covers US-016-AC1
async fn postgres_offset_store_fetch_missing() {
    if !pg_tests_enabled() {
        return;
    }
    let (_container, url) = start_postgres().await;
    tokio::task::spawn_blocking(move || {
        let store = PostgresOffsetStore::connect(&url).expect("connect");
        suites::offset_store::check_fetch_missing(store.as_ref());
    })
    .await
    .expect("task panicked");
}

#[tokio::test]
// @covers US-016-AC1
async fn postgres_offset_store_commit_and_fetch() {
    if !pg_tests_enabled() {
        return;
    }
    let (_container, url) = start_postgres().await;
    tokio::task::spawn_blocking(move || {
        let store = PostgresOffsetStore::connect(&url).expect("connect");
        suites::offset_store::check_commit_and_fetch(store.as_ref());
    })
    .await
    .expect("task panicked");
}

#[tokio::test]
// @covers US-016-AC1
async fn postgres_offset_store_commit_overwrites() {
    if !pg_tests_enabled() {
        return;
    }
    let (_container, url) = start_postgres().await;
    tokio::task::spawn_blocking(move || {
        let store = PostgresOffsetStore::connect(&url).expect("connect");
        suites::offset_store::check_commit_overwrites(store.as_ref());
    })
    .await
    .expect("task panicked");
}

#[tokio::test]
// @covers US-016-AC1
async fn postgres_offset_store_fetch_all_for_group() {
    if !pg_tests_enabled() {
        return;
    }
    let (_container, url) = start_postgres().await;
    tokio::task::spawn_blocking(move || {
        let store = PostgresOffsetStore::connect(&url).expect("connect");
        suites::offset_store::check_fetch_all_for_group(store.as_ref());
    })
    .await
    .expect("task panicked");
}

#[tokio::test]
// @covers US-016-AC1
async fn postgres_offset_store_delete_group() {
    if !pg_tests_enabled() {
        return;
    }
    let (_container, url) = start_postgres().await;
    tokio::task::spawn_blocking(move || {
        let store = PostgresOffsetStore::connect(&url).expect("connect");
        suites::offset_store::check_delete_group(store.as_ref());
    })
    .await
    .expect("task panicked");
}
