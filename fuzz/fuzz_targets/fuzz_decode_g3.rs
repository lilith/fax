#![no_main]
use libfuzzer_sys::fuzz_target;
use fax::decoder;

fuzz_target!(|data: &[u8]| {
    let mut lines = 0u32;
    decoder::decode_g3(data.iter().copied(), |_transitions| {
        lines += 1;
        // Limit lines to prevent excessive memory/time on pathological inputs
        if lines > 10_000 {
            return;
        }
    });
});
