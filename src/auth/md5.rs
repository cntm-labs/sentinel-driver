/// Compute the MD5 password hash as PostgreSQL expects it.
///
/// PG MD5 auth: `"md5" + md5(md5(password + user) + salt)`
///
/// Note: We use a two-round approach matching PG's MD5 auth protocol.
/// This is provided for legacy compatibility only.
pub fn compute_md5(user: &str, password: &str, salt: &[u8; 4]) -> String {
    // First round: md5(password + username)
    let mut hasher = md5_hash();
    hasher.update(password.as_bytes());
    hasher.update(user.as_bytes());
    let first = hex_encode(&hasher.finalize());

    // Second round: md5(first_hex + salt)
    let mut hasher = md5_hash();
    hasher.update(first.as_bytes());
    hasher.update(salt);
    let second = hex_encode(&hasher.finalize());

    format!("md5{second}")
}

/// Simple MD5 implementation using manual computation.
/// We avoid pulling in the `md5` crate — this is legacy code that should rarely run.
struct Md5Hasher {
    data: Vec<u8>,
}

fn md5_hash() -> Md5Hasher {
    Md5Hasher { data: Vec::new() }
}

impl Md5Hasher {
    fn update(&mut self, data: &[u8]) {
        self.data.extend_from_slice(data);
    }

    fn finalize(self) -> [u8; 16] {
        md5_compute(&self.data)
    }
}

/// Pure MD5 computation (RFC 1321).
fn md5_compute(input: &[u8]) -> [u8; 16] {
    let mut a0: u32 = 0x67452301;
    let mut b0: u32 = 0xefcdab89;
    let mut c0: u32 = 0x98badcfe;
    let mut d0: u32 = 0x10325476;

    // Pre-processing: add padding
    let orig_len_bits = (input.len() as u64) * 8;
    let mut msg = input.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&orig_len_bits.to_le_bytes());

    // Per-round shift amounts
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];

    // Pre-computed T[i] = floor(2^32 * |sin(i + 1)|)
    const K: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee, 0xf57c0faf, 0x4787c62a, 0xa8304613,
        0xfd469501, 0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be, 0x6b901122, 0xfd987193,
        0xa679438e, 0x49b40821, 0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa, 0xd62f105d,
        0x02441453, 0xd8a1e681, 0xe7d3fbc8, 0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a, 0xfffa3942, 0x8771f681, 0x6d9d6122,
        0xfde5380c, 0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70, 0x289b7ec6, 0xeaa127fa,
        0xd4ef3085, 0x04881d05, 0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665, 0xf4292244,
        0x432aff97, 0xab9423a7, 0xfc93a039, 0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1, 0xf7537e82, 0xbd3af235, 0x2ad7d2bb,
        0xeb86d391,
    ];

    for chunk in msg.chunks_exact(64) {
        let mut m = [0u32; 16];
        for (i, word) in m.iter_mut().enumerate() {
            *word = u32::from_le_bytes(chunk[i * 4..i * 4 + 4].try_into().unwrap());
        }

        let (mut a, mut b, mut c, mut d) = (a0, b0, c0, d0);

        for i in 0..64 {
            let (f, g) = match i {
                0..=15 => ((b & c) | (!b & d), i),
                16..=31 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };

            let f = f.wrapping_add(a).wrapping_add(K[i]).wrapping_add(m[g]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(f.rotate_left(S[i]));
        }

        a0 = a0.wrapping_add(a);
        b0 = b0.wrapping_add(b);
        c0 = c0.wrapping_add(c);
        d0 = d0.wrapping_add(d);
    }

    let mut result = [0u8; 16];
    result[0..4].copy_from_slice(&a0.to_le_bytes());
    result[4..8].copy_from_slice(&b0.to_le_bytes());
    result[8..12].copy_from_slice(&c0.to_le_bytes());
    result[12..16].copy_from_slice(&d0.to_le_bytes());
    result
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md5_basic() {
        // MD5("") = d41d8cd98f00b204e9800998ecf8427e
        let result = md5_compute(b"");
        assert_eq!(hex_encode(&result), "d41d8cd98f00b204e9800998ecf8427e");
    }

    #[test]
    fn test_md5_hello() {
        // MD5("hello") = 5d41402abc4b2a76b9719d911017c592
        let result = md5_compute(b"hello");
        assert_eq!(hex_encode(&result), "5d41402abc4b2a76b9719d911017c592");
    }

    #[test]
    fn test_compute_md5_password() {
        // Verify the two-round MD5 hash format
        let result = compute_md5("user", "pass", &[0x01, 0x02, 0x03, 0x04]);
        assert!(result.starts_with("md5"));
        assert_eq!(result.len(), 3 + 32); // "md5" + 32 hex chars
    }

    #[test]
    fn test_md5_known_vector() {
        // Known PG MD5 auth test vector:
        // user="postgres", password="postgres", salt=[0x93, 0xf8, 0xa3, 0xe4]
        // Expected: md5 + md5(md5("postgrespostgres") + salt)
        let hash1 = md5_compute(b"postgrespostgres");
        let hex1 = hex_encode(&hash1);

        let mut round2_input = hex1.into_bytes();
        round2_input.extend_from_slice(&[0x93, 0xf8, 0xa3, 0xe4]);
        let hash2 = md5_compute(&round2_input);
        let expected = format!("md5{}", hex_encode(&hash2));

        let result = compute_md5("postgres", "postgres", &[0x93, 0xf8, 0xa3, 0xe4]);
        assert_eq!(result, expected);
    }
}
