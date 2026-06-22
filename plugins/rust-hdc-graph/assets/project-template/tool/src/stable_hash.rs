use crate::hdc::tokenize;

const OFFSET_A: u64 = 0xcbf29ce484222325;
const PRIME_A: u64 = 0x100000001b3;
const OFFSET_B: u64 = 0x84222325cbf29ce4;
const PRIME_B: u64 = 0x100000001b1;
const PART_SEPARATOR: u8 = 0x1f;

pub fn stable_hash_hex<I, S>(parts: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut left = OFFSET_A;
    let mut right = OFFSET_B;

    for part in parts {
        let bytes = part.as_ref().as_bytes();
        for byte in bytes {
            left ^= *byte as u64;
            left = left.wrapping_mul(PRIME_A);

            right ^= (!*byte) as u64;
            right = right.wrapping_mul(PRIME_B);
        }

        left ^= PART_SEPARATOR as u64;
        left = left.wrapping_mul(PRIME_A);
        right ^= (!PART_SEPARATOR) as u64;
        right = right.wrapping_mul(PRIME_B);
    }

    format!("{left:016x}{right:016x}")
}

pub fn stable_text_hash(text: &str) -> String {
    stable_hash_hex([text])
}

pub fn normalize_query(text: &str) -> String {
    tokenize(text).join(" ")
}
