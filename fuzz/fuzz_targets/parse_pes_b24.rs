#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| arib_caption_worker::fuzzing::parse_pes_b24_headers(data));
