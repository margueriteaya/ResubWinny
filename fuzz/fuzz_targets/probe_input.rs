#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| arib_caption_worker::fuzzing::probe_input(data));
