//! Fuzz the full request path: decode + dispatch arbitrary bytes through the
//! router against in-memory backends. The broker parses untrusted bytes on every
//! connection, so `route` must never panic — only return Ok/Err.

#![no_main]

use heimq::protocol::Router;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Fresh in-memory broker per input so state never accumulates across the
    // fuzzer's iterations (avoids OOM over millions of runs).
    let config = heimq::test_support::test_config(true);
    let storage = heimq::test_support::test_storage(true);
    let groups = heimq::test_support::test_consumer_groups(config.clone());
    let cluster = heimq::storage::SingleNodeClusterView::arc_from_config(&config);
    let router = Router::new(storage, groups, cluster);

    // Must not panic on any input.
    let _ = router.route(data);
});
