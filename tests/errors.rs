use fax::{decoder, decoder::pels, Color, VecWriter, BitWriter, Bits};
use std::fs;
use std::path::Path;

fn parse_filename(name: &str) -> Option<(&str, u16)> {
    let name = name.strip_suffix(".raw")?;
    let (id, rest) = name.split_once('_')?;
    let width_str = rest.strip_prefix("0-w")?;
    let width = width_str.parse().ok()?;
    Some((id, width))
}

/// Decode every .raw file in test-files/errors/ and verify basic sanity.
/// These are real-world G4 images from issue #5 that previously caused
/// decode failures.
#[test]
fn decode_error_files() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-files/errors");

    let mut tested = 0;
    let mut failures = vec![];
    for entry in fs::read_dir(&dir).expect("test-files/errors/ not found") {
        let entry = entry.unwrap();
        let path = entry.path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();

        let (id, width) = match parse_filename(&name) {
            Some(v) => v,
            None => continue,
        };

        let data = fs::read(&path).unwrap();
        let mut lines = 0u32;
        let mut max_transition = 0u16;
        let result = decoder::decode_g4(data.iter().copied(), width, None, |transitions| {
            lines += 1;
            if let Some(&last) = transitions.last() {
                if last > max_transition {
                    max_transition = last;
                }
            }
        });

        if result.is_none() {
            failures.push(format!("{id} (w={width}): decode returned None after {lines} lines"));
        } else if lines == 0 {
            failures.push(format!("{id} (w={width}): decoded 0 lines"));
        } else if max_transition > width {
            failures.push(format!(
                "{id} (w={width}): max transition {max_transition} exceeds width"
            ));
        }
        tested += 1;
    }

    assert!(tested > 0, "no .raw files found in test-files/errors/");
    assert!(
        failures.is_empty(),
        "{} of {tested} error files failed:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}

/// Roundtrip: decode each .raw, encode the decoded pels, decode again,
/// compare pixel-by-pixel. This catches encoder/decoder asymmetries.
#[test]
fn roundtrip_error_files() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-files/errors");

    let mut tested = 0;
    let mut failures = vec![];
    for entry in fs::read_dir(&dir).expect("test-files/errors/ not found") {
        let entry = entry.unwrap();
        let path = entry.path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();

        let (id, width) = match parse_filename(&name) {
            Some(v) => v,
            None => continue,
        };

        let data = fs::read(&path).unwrap();

        // Decode original
        let mut orig_lines: Vec<Vec<Color>> = Vec::new();
        let ok = decoder::decode_g4(data.iter().copied(), width, None, |transitions| {
            orig_lines.push(pels(transitions, width).collect());
        });
        if ok.is_none() || orig_lines.is_empty() {
            continue; // skip files that don't decode (tested above)
        }

        // Encode from decoded pels
        let height = orig_lines.len() as u16;
        let writer = VecWriter::new();
        let mut encoder = fax::encoder::Encoder::new(writer);
        for line in &orig_lines {
            let _ = encoder.encode_line(line.iter().copied(), width);
        }
        let encoded = encoder.finish().unwrap().finish();

        // Decode re-encoded data
        let mut rt_lines: Vec<Vec<Color>> = Vec::new();
        let rt_ok =
            decoder::decode_g4(encoded.iter().copied(), width, Some(height), |transitions| {
                rt_lines.push(pels(transitions, width).collect());
            });

        if rt_ok.is_none() {
            failures.push(format!("{id} (w={width}): roundtrip decode returned None"));
            tested += 1;
            continue;
        }
        if rt_lines.len() != orig_lines.len() {
            failures.push(format!(
                "{id} (w={width}): roundtrip line count {}, expected {}",
                rt_lines.len(),
                orig_lines.len()
            ));
            tested += 1;
            continue;
        }
        for (i, (orig, rt)) in orig_lines.iter().zip(rt_lines.iter()).enumerate() {
            if orig != rt {
                failures.push(format!("{id} (w={width}): roundtrip mismatch at line {i}"));
                break;
            }
        }
        tested += 1;
    }

    assert!(tested > 0, "no .raw files found in test-files/errors/");
    assert!(
        failures.is_empty(),
        "{} of {tested} error files failed roundtrip:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}
