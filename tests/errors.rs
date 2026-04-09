use fax::{decoder, decoder::pels, BitWriter, Bits, Color, VecWriter};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::io::Write;
use std::path::Path;

fn parse_raw_filename(name: &str) -> Option<(&str, u16)> {
    let name = name.strip_suffix(".raw")?;
    let (id, rest) = name.split_once('_')?;
    let width_str = rest.strip_prefix("0-w")?;
    let width = width_str.parse().ok()?;
    Some((id, width))
}

struct RefEntry {
    dir: String,
    width: u16,
    height: u16,
    sha256: String,
}

fn load_reference_hashes() -> HashMap<String, RefEntry> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-files/reference-hashes.tsv");
    let content = fs::read_to_string(&path).expect("test-files/reference-hashes.tsv not found");
    let mut map = HashMap::new();
    for line in content.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        assert!(
            fields.len() == 5,
            "bad line in reference-hashes.tsv: {line}"
        );
        map.insert(
            format!("{}:{}", fields[1], fields[0]),
            RefEntry {
                dir: fields[1].to_string(),
                width: fields[2].parse().unwrap(),
                height: fields[3].parse().unwrap(),
                sha256: fields[4].to_string(),
            },
        );
    }
    map
}

/// Render decoded transitions to PBM bytes (P4 format).
fn decode_to_pbm(data: &[u8], width: u16, height: u16) -> Option<Vec<u8>> {
    let mut pbm = Vec::new();
    write!(pbm, "P4\n{width} {height}\n").unwrap();
    let mut lines = 0u16;
    let ok = decoder::decode_g4(data.iter().copied(), width, Some(height), |transitions| {
        let mut writer = VecWriter::new();
        for c in pels(transitions, width) {
            let bit = match c {
                Color::Black => Bits { data: 1, len: 1 },
                Color::White => Bits { data: 0, len: 1 },
            };
            writer.write(bit).unwrap();
        }
        writer.pad();
        pbm.extend(writer.finish());
        lines += 1;
    });
    if ok.is_some() && lines == height {
        Some(pbm)
    } else {
        None
    }
}

fn sha256_hex(data: &[u8]) -> String {
    let hash = Sha256::digest(data);
    let mut hex = String::with_capacity(64);
    for byte in hash {
        write!(hex, "{byte:02x}").unwrap();
    }
    hex
}

/// Compare decoded output of errors/*.raw against libtiff reference hashes.
#[test]
fn errors_match_reference_hashes() {
    let refs = load_reference_hashes();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-files/errors");

    let mut tested = 0;
    let mut failures = vec![];
    for entry in fs::read_dir(&dir).expect("test-files/errors/ not found") {
        let entry = entry.unwrap();
        let path = entry.path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();

        let (id, width) = match parse_raw_filename(&name) {
            Some(v) => v,
            None => continue,
        };

        let key = format!("errors:{id}");
        let reference = match refs.get(&key) {
            Some(r) => r,
            None => {
                failures.push(format!("{id}: no reference hash in TSV"));
                tested += 1;
                continue;
            }
        };

        assert_eq!(width, reference.width, "{id}: width mismatch with TSV");

        let data = fs::read(&path).unwrap();
        let pbm = match decode_to_pbm(&data, reference.width, reference.height) {
            Some(p) => p,
            None => {
                failures.push(format!("{id}: decode failed"));
                tested += 1;
                continue;
            }
        };

        let hash = sha256_hex(&pbm);
        if hash != reference.sha256 {
            failures.push(format!(
                "{id}: hash mismatch\n    got:    {hash}\n    expect: {}",
                reference.sha256
            ));
        }
        tested += 1;
    }

    assert!(tested > 0, "no .raw files found");
    assert!(
        failures.is_empty(),
        "{} of {tested} files failed reference hash check:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}

/// Compare decoded output of files/*.fax and files/*.tiff against reference hashes.
#[test]
fn files_match_reference_hashes() {
    let refs = load_reference_hashes();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-files/files");

    let mut tested = 0;
    let mut failures = vec![];
    for entry in fs::read_dir(&dir).expect("test-files/files/ not found") {
        let entry = entry.unwrap();
        let p = entry.path();
        let stem = p.file_stem().unwrap().to_string_lossy().to_string();

        let (stream, white_is_1) = if p.extension().is_some_and(|e| e == "fax") {
            (fs::read(&p).unwrap(), false)
        } else if p.extension().is_some_and(|e| e == "tiff") {
            use tiff::{decoder::Decoder, tags::Tag};
            let data = fs::read(&p).unwrap();
            let reader = std::io::Cursor::new(data.as_slice());
            let mut dec = Decoder::new(reader).unwrap();
            let off = dec
                .get_tag(Tag::StripOffsets)
                .unwrap()
                .into_u32()
                .unwrap() as usize;
            let len = dec
                .get_tag(Tag::StripByteCounts)
                .unwrap()
                .into_u32()
                .unwrap() as usize;
            let w1 = dec
                .get_tag(Tag::PhotometricInterpretation)
                .unwrap()
                .into_u16()
                .unwrap()
                != 0;
            (data[off..off + len].to_vec(), w1)
        } else {
            continue;
        };

        let key = format!("files:{stem}");
        let reference = match refs.get(&key) {
            Some(r) => r,
            None => {
                failures.push(format!("{stem}: no reference hash in TSV"));
                tested += 1;
                continue;
            }
        };

        // Decode to PBM, respecting white_is_1
        let (black_bit, white_bit) = if white_is_1 {
            (
                Bits { data: 0, len: 1 },
                Bits { data: 1, len: 1 },
            )
        } else {
            (
                Bits { data: 1, len: 1 },
                Bits { data: 0, len: 1 },
            )
        };

        let mut pbm = Vec::new();
        write!(
            pbm,
            "P4\n{} {}\n",
            reference.width, reference.height
        )
        .unwrap();
        let mut lines = 0u16;
        let ok = decoder::decode_g4(
            stream.iter().copied(),
            reference.width,
            Some(reference.height),
            |transitions| {
                let mut writer = VecWriter::new();
                for c in pels(transitions, reference.width) {
                    let bit = match c {
                        Color::Black => black_bit,
                        Color::White => white_bit,
                    };
                    writer.write(bit).unwrap();
                }
                writer.pad();
                pbm.extend(writer.finish());
                lines += 1;
            },
        );

        if ok.is_none() || lines != reference.height {
            failures.push(format!(
                "{stem}: decode failed ({lines}/{} lines)",
                reference.height
            ));
            tested += 1;
            continue;
        }

        let hash = sha256_hex(&pbm);
        if hash != reference.sha256 {
            failures.push(format!(
                "{stem}: hash mismatch\n    got:    {hash}\n    expect: {}",
                reference.sha256
            ));
        }
        tested += 1;
    }

    assert!(tested > 0, "no test images found");
    assert!(
        failures.is_empty(),
        "{} of {tested} files failed reference hash check:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}

/// Roundtrip: decode → encode → decode → compare pels.
#[test]
fn roundtrip_error_files() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-files/errors");

    let mut tested = 0;
    let mut failures = vec![];
    for entry in fs::read_dir(&dir).expect("test-files/errors/ not found") {
        let entry = entry.unwrap();
        let path = entry.path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();

        let (id, width) = match parse_raw_filename(&name) {
            Some(v) => v,
            None => continue,
        };

        let data = fs::read(&path).unwrap();

        let mut orig_lines: Vec<Vec<Color>> = Vec::new();
        let ok = decoder::decode_g4(data.iter().copied(), width, None, |transitions| {
            orig_lines.push(pels(transitions, width).collect());
        });
        if ok.is_none() || orig_lines.is_empty() {
            continue;
        }

        let height = orig_lines.len() as u16;
        let writer = VecWriter::new();
        let mut encoder = fax::encoder::Encoder::new(writer);
        for line in &orig_lines {
            let _ = encoder.encode_line(line.iter().copied(), width);
        }
        let encoded = encoder.finish().unwrap().finish();

        let mut rt_lines: Vec<Vec<Color>> = Vec::new();
        let rt_ok =
            decoder::decode_g4(encoded.iter().copied(), width, Some(height), |transitions| {
                rt_lines.push(pels(transitions, width).collect());
            });

        if rt_ok.is_none() {
            failures.push(format!("{id}: roundtrip decode returned None"));
        } else if rt_lines.len() != orig_lines.len() {
            failures.push(format!(
                "{id}: roundtrip line count {}, expected {}",
                rt_lines.len(),
                orig_lines.len()
            ));
        } else {
            for (i, (orig, rt)) in orig_lines.iter().zip(rt_lines.iter()).enumerate() {
                if orig != rt {
                    failures.push(format!("{id}: roundtrip mismatch at line {i}"));
                    break;
                }
            }
        }
        tested += 1;
    }

    assert!(tested > 0, "no .raw files found");
    assert!(
        failures.is_empty(),
        "{} of {tested} files failed roundtrip:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}
