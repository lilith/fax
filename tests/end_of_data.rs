//! Codes in the last bits of the input must decode: the code tables peek up to 16 bits, and
//! data that ends without EOFB (as TIFF strips may) leaves fewer than that after the last code.

use fax::{decoder, encoder::Encoder, BitWriter, Bits, Color};

/// Records written bits so the end-of-block marker can be removed.
#[derive(Default)]
struct Recorder(Vec<bool>);

impl BitWriter for Recorder {
    type Error = std::convert::Infallible;
    fn write(&mut self, bits: Bits) -> Result<(), Self::Error> {
        for i in (0..bits.len).rev() {
            self.0.push((bits.data >> i) & 1 == 1);
        }
        Ok(())
    }
}

fn rows(width: u32, height: usize, seed: u32) -> Vec<Vec<Color>> {
    let mut state = seed.wrapping_mul(2654435761) | 1;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        state
    };
    (0..height)
        .map(|_| {
            let mut row = Vec::new();
            let mut color = if next() % 2 == 0 {
                Color::White
            } else {
                Color::Black
            };
            while row.len() < width as usize {
                let n = (next() % 9 + 1) as usize;
                row.extend(std::iter::repeat(color).take(n.min(width as usize - row.len())));
                color = !color;
            }
            row
        })
        .collect()
}

#[test]
fn g4_without_eofb_decodes_last_row() {
    let mut failures = Vec::new();
    for seed in 0..200 {
        let width = 1 + seed % 40;
        let input = rows(width, 3, seed);
        let mut encoder = Encoder::new(Recorder::default());
        for row in &input {
            encoder.encode_line(row.iter().copied(), width).unwrap();
        }
        // Drop the 24-bit EOFB that `finish` appends: the data ends right after the last
        // row's codes, padded with zero bits to a byte.
        let mut bits = encoder.finish().unwrap().0;
        bits.truncate(bits.len() - 24);
        let data: Vec<u8> = bits
            .chunks(8)
            .map(|c| {
                c.iter()
                    .enumerate()
                    .fold(0u8, |b, (i, &v)| b | ((v as u8) << (7 - i)))
            })
            .collect();

        let mut decoded = Vec::new();
        decoder::decode_g4(data.into_iter(), width, Some(input.len() as u32), |t| {
            decoded.push(decoder::pels(t, width).collect::<Vec<_>>());
        });
        if decoded != input {
            failures.push(seed);
        }
    }
    assert!(failures.is_empty(), "rows differ for seeds {:?}", failures);
}
