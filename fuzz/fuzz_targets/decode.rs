//! Fuzz the wire-format request decoder in isolation (fast, parser-only). This is
//! the first code that touches untrusted bytes on every connection.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = heimq::protocol::decode_request(data);
});
