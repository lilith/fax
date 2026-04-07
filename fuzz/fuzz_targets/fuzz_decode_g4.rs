#![no_main]
use libfuzzer_sys::fuzz_target;
use fax::decoder;

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }

    // Use first 2 bytes as width (1..=4096), next 2 as height (optional)
    let width = u16::from_le_bytes([data[0], data[1]]);
    let width = (width % 4096).max(1);

    let height_raw = u16::from_le_bytes([data[2], data[3]]);
    let height = if height_raw == 0 {
        None
    } else {
        Some((height_raw % 4096).max(1))
    };

    let payload = &data[4..];

    let mut lines = 0u32;
    decoder::decode_g4(payload.iter().copied(), width, height, |_transitions| {
        lines += 1;
        if lines > 10_000 {
            return;
        }
    });
});
