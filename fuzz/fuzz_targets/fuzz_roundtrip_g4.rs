#![no_main]

use libfuzzer_sys::fuzz_target;

// Roundtrip fuzzer: generate a pixel line, encode as G4, decode back,
// verify the decoded pixels match the input.
//
// Important: transition lists are NOT a canonical representation. For
// example, `[3]` and `[3, 4]` both represent "3W 1B" at width=4. The
// encoder and decoder must agree on the resulting pixels, but their
// internal transition lists may differ in whether they include the
// width-boundary sentinel. We compare pels (pixel colors) — the
// semantic form — not transition lists.
//
// Canonical form for this crate: transitions are strictly in [0, width),
// never equal to width. The decoder must preserve this invariant.
fuzz_target!(|data: &[u8]| {
    if data.len() < 3 {
        return;
    }
    let width = u16::from_le_bytes([data[0], data[1]]).max(1).min(2000);
    let remaining = &data[2..];

    // Build transition list from fuzz input (monotonic, within [0, width))
    let mut transitions: Vec<u16> = Vec::new();
    let mut pos = 0u16;
    for &b in remaining {
        let step = (b as u16).min(width.saturating_sub(1).saturating_sub(pos));
        if step == 0 {
            break;
        }
        pos = pos.saturating_add(step);
        if pos >= width {
            break;
        }
        transitions.push(pos);
    }

    // Expected pels from the input
    let input_pels: Vec<_> = fax::decoder::pels(&transitions, width).collect();

    // Encode
    let writer = fax::VecWriter::new();
    let mut encoder = fax::encoder::Encoder::new(writer);
    let _ = encoder.encode_line(input_pels.iter().copied(), width);
    let encoded = match encoder.finish() {
        Ok(w) => w.finish(),
        Err(_) => return,
    };

    // Decode back
    let mut decoded_lines = Vec::new();
    let _ = fax::decoder::decode_g4(encoded.into_iter(), width, Some(1), |line| {
        decoded_lines.push(line.to_vec());
    });

    // Verify: decoded pels must match input pels (semantic equality).
    if let Some(decoded) = decoded_lines.first() {
        // Canonical form invariant: decoder output transitions must all be < width.
        assert!(
            decoded.iter().all(|&t| t < width),
            "decoder produced non-canonical transition list {:?} at width={} \
             (contains width sentinel); expected all transitions < width",
            decoded,
            width
        );
        let output_pels: Vec<_> = fax::decoder::pels(decoded, width).collect();
        assert_eq!(
            input_pels, output_pels,
            "G4 roundtrip pels mismatch: width={}, input transitions={:?}, \
             decoded transitions={:?}",
            width, transitions, decoded
        );
    }
});
