#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| arib_caption_worker::fuzzing::decode_arib_si_text(data));
