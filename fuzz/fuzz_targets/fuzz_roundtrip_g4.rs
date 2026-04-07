#![no_main]

use libfuzzer_sys::fuzz_target;

/// Roundtrip fuzzer: generate transitions, encode as G4, decode back.
/// Verifies encode/decode consistency.
fuzz_target!(|data: &[u8]| {
    if data.len() < 3 {
        return;
    }
    let width = u16::from_le_bytes([data[0], data[1]]).max(1).min(2000);
    let remaining = &data[2..];

    // Build transition list from fuzz input (sorted, within width)
    let mut transitions: Vec<u16> = Vec::new();
    let mut pos = 0u16;
    for &b in remaining {
        let step = (b as u16).min(width - pos);
        if step == 0 { break; }
        pos = pos.saturating_add(step);
        if pos > width { break; }
        transitions.push(pos);
        if pos >= width { break; }
    }

    // Encode
    let mut writer = fax::VecWriter::new();
    let mut encoder = fax::Encoder::new(&mut writer);
    let _ = encoder.encode_line(&transitions, width);
    let encoded = match encoder.finish() {
        Ok(w) => w.finish(),
        Err(_) => return,
    };

    // Decode back
    let mut decoded_lines = Vec::new();
    let _ = fax::decode_g4(encoded.into_iter(), width, Some(1), |line| {
        decoded_lines.push(line.to_vec());
    });

    // If we got a line back, verify transitions match
    if let Some(decoded) = decoded_lines.first() {
        assert_eq!(
            &transitions, decoded,
            "G4 roundtrip mismatch: width={}, encoded transitions={:?}",
            width, transitions
        );
    }
});
