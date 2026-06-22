use std::fmt::Write;

pub const HD_BITS: usize = 512;
const HD_WORDS: usize = HD_BITS / 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HyperVector {
    pub words: [u64; HD_WORDS],
}

impl HyperVector {
    pub fn zero() -> Self {
        Self {
            words: [0; HD_WORDS],
        }
    }

    pub fn seeded(seed: &str) -> Self {
        let mut words = [0u64; HD_WORDS];
        for (idx, word) in words.iter_mut().enumerate() {
            *word = fnv1a64(&format!("{seed}::{idx}"));
        }
        Self { words }
    }

    pub fn xor(self, other: Self) -> Self {
        let mut words = [0u64; HD_WORDS];
        for (idx, word) in words.iter_mut().enumerate() {
            *word = self.words[idx] ^ other.words[idx];
        }
        Self { words }
    }

    pub fn rotate_left(self, amount: usize) -> Self {
        let shift = amount % HD_BITS;
        if shift == 0 {
            return self;
        }

        let word_shift = shift / 64;
        let bit_shift = shift % 64;
        let mut words = [0u64; HD_WORDS];

        for idx in 0..HD_WORDS {
            let low_idx = (idx + HD_WORDS - word_shift) % HD_WORDS;
            let high_idx = (idx + HD_WORDS - word_shift - 1) % HD_WORDS;

            let low = self.words[low_idx];
            let high = self.words[high_idx];

            words[idx] = if bit_shift == 0 {
                low
            } else {
                (low << bit_shift) | (high >> (64 - bit_shift))
            };
        }

        Self { words }
    }

    pub fn bundle(vectors: &[Self]) -> Self {
        if vectors.is_empty() {
            return Self::zero();
        }

        let mut counts = [0i32; HD_BITS];
        for vector in vectors {
            for bit in 0..HD_BITS {
                let set = ((vector.words[bit / 64] >> (bit % 64)) & 1) == 1;
                counts[bit] += if set { 1 } else { -1 };
            }
        }

        let mut words = [0u64; HD_WORDS];
        for bit in 0..HD_BITS {
            if counts[bit] >= 0 {
                words[bit / 64] |= 1u64 << (bit % 64);
            }
        }

        Self { words }
    }

    pub fn similarity(self, other: Self) -> f64 {
        let diff_bits: u32 = self
            .words
            .iter()
            .zip(other.words.iter())
            .map(|(left, right)| (left ^ right).count_ones())
            .sum();
        1.0 - (diff_bits as f64 / HD_BITS as f64)
    }

    pub fn from_tokens(tokens: &[String]) -> Self {
        let mut vectors = Vec::new();
        for (idx, token) in tokens.iter().enumerate() {
            if token.is_empty() {
                continue;
            }
            vectors.push(Self::seeded(token).rotate_left(idx * 17));
        }
        Self::bundle(&vectors)
    }

    pub fn relation(source: Self, relation: &str, target: Self) -> Self {
        let role_src = Self::seeded("role:source");
        let role_rel = Self::seeded("role:relation");
        let role_dst = Self::seeded("role:target");
        let rel_vec = Self::seeded(&format!("rel:{relation}"));

        role_src
            .xor(source.rotate_left(11))
            .xor(role_rel.xor(rel_vec.rotate_left(29)))
            .xor(role_dst.xor(target.rotate_left(47)))
    }

    pub fn to_hex(self) -> String {
        let mut out = String::with_capacity(HD_WORDS * 16);
        for word in self.words {
            let _ = write!(&mut out, "{word:016x}");
        }
        out
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        if hex.len() != HD_WORDS * 16 {
            return None;
        }

        let mut words = [0u64; HD_WORDS];
        for (idx, chunk) in hex.as_bytes().chunks(16).enumerate() {
            let piece = std::str::from_utf8(chunk).ok()?;
            words[idx] = u64::from_str_radix(piece, 16).ok()?;
        }
        Some(Self { words })
    }
}

pub fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let chars = text.chars().collect::<Vec<_>>();

    for (idx, ch) in chars.iter().copied().enumerate() {
        if ch.is_ascii_alphanumeric() {
            let prev = idx.checked_sub(1).and_then(|prev| chars.get(prev)).copied();
            let next = chars.get(idx + 1).copied();

            let boundary = prev.is_some_and(|prev| {
                (ch.is_ascii_digit() && prev.is_ascii_alphabetic())
                    || (ch.is_ascii_alphabetic() && prev.is_ascii_digit())
                    || (ch.is_ascii_uppercase() && prev.is_ascii_lowercase())
                    || (ch.is_ascii_uppercase()
                        && prev.is_ascii_uppercase()
                        && next.is_some_and(|next| next.is_ascii_lowercase()))
            });

            if boundary && !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            current.push(ch.to_ascii_lowercase());
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn fnv1a64(input: &str) -> u64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;

    let mut hash = OFFSET;
    for byte in input.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::{HyperVector, tokenize};

    #[test]
    fn rotation_preserves_similarity_shape() {
        let vector = HyperVector::seeded("compute");
        let rotated = vector.rotate_left(19);
        assert!(vector.similarity(rotated) < 0.75);
    }

    #[test]
    fn bundle_is_stable() {
        let a = HyperVector::seeded("a");
        let b = HyperVector::seeded("b");
        let c = HyperVector::bundle(&[a, b]);
        assert!(c.similarity(a) > 0.45);
        assert!(c.similarity(b) > 0.45);
    }

    #[test]
    fn hex_round_trip_works() {
        let vector = HyperVector::seeded("roundtrip");
        let recovered = HyperVector::from_hex(&vector.to_hex()).unwrap();
        assert_eq!(vector, recovered);
    }

    #[test]
    fn tokenize_splits_camel_case() {
        assert_eq!(
            tokenize("ChartGraph fieldGradientSummary HTTPServer2"),
            vec![
                "chart".to_string(),
                "graph".to_string(),
                "field".to_string(),
                "gradient".to_string(),
                "summary".to_string(),
                "http".to_string(),
                "server".to_string(),
                "2".to_string(),
            ]
        );
    }
}
