#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Adler32 {
    a: u32,
    b: u32,
}

impl Default for Adler32 {
    fn default() -> Self {
        Self::new()
    }
}

impl Adler32 {
    const MOD_ADLER: u32 = 65_521;

    pub fn new() -> Self {
        Self { a: 1, b: 0 }
    }

    pub fn update(&mut self, data: &[u8]) {
        for byte in data {
            self.a = (self.a + u32::from(*byte)) % Self::MOD_ADLER;
            self.b = (self.b + self.a) % Self::MOD_ADLER;
        }
    }

    pub fn finalize(self) -> u32 {
        (self.b << 16) | self.a
    }
}

pub fn adler32(data: &[u8]) -> u32 {
    let mut state = Adler32::new();
    state.update(data);
    state.finalize()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Crc32 {
    state: u32,
}

impl Default for Crc32 {
    fn default() -> Self {
        Self::new()
    }
}

impl Crc32 {
    const POLY_REVERSED: u32 = 0xedb8_8320;

    pub fn new() -> Self {
        Self { state: 0xffff_ffff }
    }

    pub fn update(&mut self, data: &[u8]) {
        for byte in data {
            self.state ^= u32::from(*byte);
            for _ in 0..8 {
                let mask = 0_u32.wrapping_sub(self.state & 1);
                self.state = (self.state >> 1) ^ (Self::POLY_REVERSED & mask);
            }
        }
    }

    pub fn finalize(self) -> u32 {
        !self.state
    }
}

pub fn crc32_ieee(data: &[u8]) -> u32 {
    let mut state = Crc32::new();
    state.update(data);
    state.finalize()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Md5 {
    state: [u32; 4],
    len_bytes: u64,
    buffer: [u8; 64],
    buffer_len: usize,
}

impl Default for Md5 {
    fn default() -> Self {
        Self::new()
    }
}

impl Md5 {
    const INIT: [u32; 4] = [0x6745_2301, 0xefcd_ab89, 0x98ba_dcfe, 0x1032_5476];
    const SHIFTS: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5,
        9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10,
        15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    const K: [u32; 64] = [
        0xd76a_a478,
        0xe8c7_b756,
        0x2420_70db,
        0xc1bd_ceee,
        0xf57c_0faf,
        0x4787_c62a,
        0xa830_4613,
        0xfd46_9501,
        0x6980_98d8,
        0x8b44_f7af,
        0xffff_5bb1,
        0x895c_d7be,
        0x6b90_1122,
        0xfd98_7193,
        0xa679_438e,
        0x49b4_0821,
        0xf61e_2562,
        0xc040_b340,
        0x265e_5a51,
        0xe9b6_c7aa,
        0xd62f_105d,
        0x0244_1453,
        0xd8a1_e681,
        0xe7d3_fbc8,
        0x21e1_cde6,
        0xc337_07d6,
        0xf4d5_0d87,
        0x455a_14ed,
        0xa9e3_e905,
        0xfcef_a3f8,
        0x676f_02d9,
        0x8d2a_4c8a,
        0xfffa_3942,
        0x8771_f681,
        0x6d9d_6122,
        0xfde5_380c,
        0xa4be_ea44,
        0x4bde_cfa9,
        0xf6bb_4b60,
        0xbebf_bc70,
        0x289b_7ec6,
        0xeaa1_27fa,
        0xd4ef_3085,
        0x0488_1d05,
        0xd9d4_d039,
        0xe6db_99e5,
        0x1fa2_7cf8,
        0xc4ac_5665,
        0xf429_2244,
        0x432a_ff97,
        0xab94_23a7,
        0xfc93_a039,
        0x655b_59c3,
        0x8f0c_cc92,
        0xffef_f47d,
        0x8584_5dd1,
        0x6fa8_7e4f,
        0xfe2c_e6e0,
        0xa301_4314,
        0x4e08_11a1,
        0xf753_7e82,
        0xbd3a_f235,
        0x2ad7_d2bb,
        0xeb86_d391,
    ];

    pub fn new() -> Self {
        Self {
            state: Self::INIT,
            len_bytes: 0,
            buffer: [0; 64],
            buffer_len: 0,
        }
    }

    pub fn update(&mut self, mut data: &[u8]) {
        self.len_bytes = self.len_bytes.wrapping_add(data.len() as u64);

        if self.buffer_len > 0 {
            let fill = (64 - self.buffer_len).min(data.len());
            self.buffer[self.buffer_len..self.buffer_len + fill].copy_from_slice(&data[..fill]);
            self.buffer_len += fill;
            data = &data[fill..];

            if self.buffer_len == 64 {
                let block = self.buffer;
                self.compress(&block);
                self.buffer_len = 0;
            }
        }

        let mut chunks = data.chunks_exact(64);
        for chunk in &mut chunks {
            let mut block = [0; 64];
            block.copy_from_slice(chunk);
            self.compress(&block);
        }

        let remainder = chunks.remainder();
        if !remainder.is_empty() {
            self.buffer[..remainder.len()].copy_from_slice(remainder);
            self.buffer_len = remainder.len();
        }
    }

    pub fn finalize(mut self) -> [u8; 16] {
        let bit_len = self.len_bytes.wrapping_mul(8);
        let mut padding = [0_u8; 72];
        padding[0] = 0x80;
        let pad_len = if self.buffer_len < 56 {
            56 - self.buffer_len
        } else {
            120 - self.buffer_len
        };
        self.update(&padding[..pad_len]);

        let length_bytes = bit_len.to_le_bytes();
        self.update(&length_bytes);

        let mut digest = [0_u8; 16];
        for (chunk, word) in digest.chunks_exact_mut(4).zip(self.state) {
            chunk.copy_from_slice(&word.to_le_bytes());
        }
        digest
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut words = [0_u32; 16];
        for (word, chunk) in words.iter_mut().zip(block.chunks_exact(4)) {
            *word = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }

        let [mut a, mut b, mut c, mut d] = self.state;

        for i in 0..64 {
            let (f, g) = match i {
                0..=15 => ((b & c) | (!b & d), i),
                16..=31 => ((d & b) | (!d & c), (5 * i + 1) % 16),
                32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | !d), (7 * i) % 16),
            };

            let rotated = a
                .wrapping_add(f)
                .wrapping_add(Self::K[i])
                .wrapping_add(words[g])
                .rotate_left(Self::SHIFTS[i]);
            a = d;
            d = c;
            c = b;
            b = b.wrapping_add(rotated);
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
    }
}

pub fn md5(data: &[u8]) -> [u8; 16] {
    let mut state = Md5::new();
    state.update(data);
    state.finalize()
}

pub fn digest_to_hex(digest: &[u8]) -> String {
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adler32_matches_standard_vectors() {
        assert_eq!(adler32(b""), 0x0000_0001);
        assert_eq!(adler32(b"123456789"), 0x091e_01de);
        assert_eq!(adler32(b"Wikipedia"), 0x11e6_0398);
    }

    #[test]
    fn adler32_streaming_matches_single_shot() {
        let mut state = Adler32::new();
        state.update(b"1234");
        state.update(b"56789");

        assert_eq!(state.finalize(), adler32(b"123456789"));
    }

    #[test]
    fn crc32_ieee_matches_standard_vectors() {
        assert_eq!(crc32_ieee(b""), 0x0000_0000);
        assert_eq!(crc32_ieee(b"123456789"), 0xcbf4_3926);
        assert_eq!(
            crc32_ieee(b"The quick brown fox jumps over the lazy dog"),
            0x414f_a339
        );
    }

    #[test]
    fn crc32_streaming_matches_single_shot() {
        let mut state = Crc32::new();
        state.update(b"The quick ");
        state.update(b"brown fox ");
        state.update(b"jumps over the lazy dog");

        assert_eq!(
            state.finalize(),
            crc32_ieee(b"The quick brown fox jumps over the lazy dog")
        );
    }

    #[test]
    fn md5_matches_standard_vectors() {
        assert_eq!(digest_to_hex(&md5(b"")), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(
            digest_to_hex(&md5(b"a")),
            "0cc175b9c0f1b6a831c399e269772661"
        );
        assert_eq!(
            digest_to_hex(&md5(b"abc")),
            "900150983cd24fb0d6963f7d28e17f72"
        );
        assert_eq!(
            digest_to_hex(&md5(b"message digest")),
            "f96b697d7cb7938d525a2f31aaf161d0"
        );
        assert_eq!(
            digest_to_hex(&md5(b"abcdefghijklmnopqrstuvwxyz")),
            "c3fcd3d76192e4007dfb496cca67e13b"
        );
        assert_eq!(
            digest_to_hex(&md5(
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
            )),
            "d174ab98d277d9f5a5611c2c9f419d9f"
        );
        assert_eq!(
            digest_to_hex(&md5(
                b"12345678901234567890123456789012345678901234567890123456789012345678901234567890"
            )),
            "57edf4a22be3c955ac49da2e2107b67a"
        );
    }

    #[test]
    fn md5_streaming_matches_single_shot_across_block_boundaries() {
        let payload = b"abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnopqrstuvwxyz0123456789";
        let mut state = Md5::new();

        state.update(&payload[..1]);
        state.update(&payload[1..63]);
        state.update(&payload[63..64]);
        state.update(&payload[64..]);

        assert_eq!(state.finalize(), md5(payload));
    }

    #[test]
    fn digest_to_hex_formats_lowercase_bytes() {
        assert_eq!(digest_to_hex(&[0x00, 0x0f, 0x10, 0xff]), "000f10ff");
    }
}
