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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sha256 {
    state: [u32; 8],
    len_bytes: u64,
    buffer: [u8; 64],
    buffer_len: usize,
}

impl Default for Sha256 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha256 {
    const INIT: [u32; 8] = [
        0x6a09_e667,
        0xbb67_ae85,
        0x3c6e_f372,
        0xa54f_f53a,
        0x510e_527f,
        0x9b05_688c,
        0x1f83_d9ab,
        0x5be0_cd19,
    ];
    const K: [u32; 64] = [
        0x428a_2f98,
        0x7137_4491,
        0xb5c0_fbcf,
        0xe9b5_dba5,
        0x3956_c25b,
        0x59f1_11f1,
        0x923f_82a4,
        0xab1c_5ed5,
        0xd807_aa98,
        0x1283_5b01,
        0x2431_85be,
        0x550c_7dc3,
        0x72be_5d74,
        0x80de_b1fe,
        0x9bdc_06a7,
        0xc19b_f174,
        0xe49b_69c1,
        0xefbe_4786,
        0x0fc1_9dc6,
        0x240c_a1cc,
        0x2de9_2c6f,
        0x4a74_84aa,
        0x5cb0_a9dc,
        0x76f9_88da,
        0x983e_5152,
        0xa831_c66d,
        0xb003_27c8,
        0xbf59_7fc7,
        0xc6e0_0bf3,
        0xd5a7_9147,
        0x06ca_6351,
        0x1429_2967,
        0x27b7_0a85,
        0x2e1b_2138,
        0x4d2c_6dfc,
        0x5338_0d13,
        0x650a_7354,
        0x766a_0abb,
        0x81c2_c92e,
        0x9272_2c85,
        0xa2bf_e8a1,
        0xa81a_664b,
        0xc24b_8b70,
        0xc76c_51a3,
        0xd192_e819,
        0xd699_0624,
        0xf40e_3585,
        0x106a_a070,
        0x19a4_c116,
        0x1e37_6c08,
        0x2748_774c,
        0x34b0_bcb5,
        0x391c_0cb3,
        0x4ed8_aa4a,
        0x5b9c_ca4f,
        0x682e_6ff3,
        0x748f_82ee,
        0x78a5_636f,
        0x84c8_7814,
        0x8cc7_0208,
        0x90be_fffa,
        0xa450_6ceb,
        0xbef9_a3f7,
        0xc671_78f2,
    ];

    pub fn new() -> Self {
        Self::with_init(Self::INIT)
    }

    fn with_init(init: [u32; 8]) -> Self {
        Self {
            state: init,
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

    pub fn finalize(self) -> [u8; 32] {
        let words = self.finalize_words();
        let mut digest = [0_u8; 32];
        for (chunk, word) in digest.chunks_exact_mut(4).zip(words) {
            chunk.copy_from_slice(&word.to_be_bytes());
        }
        digest
    }

    fn finalize_words(mut self) -> [u32; 8] {
        let bit_len = self.len_bytes.wrapping_mul(8);
        let mut padding = [0_u8; 72];
        padding[0] = 0x80;
        let pad_len = if self.buffer_len < 56 {
            56 - self.buffer_len
        } else {
            120 - self.buffer_len
        };
        self.update(&padding[..pad_len]);
        self.update(&bit_len.to_be_bytes());

        self.state
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut words = [0_u32; 64];
        for (word, chunk) in words.iter_mut().zip(block.chunks_exact(4)) {
            *word = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for i in 16..64 {
            let s0 = words[i - 15].rotate_right(7)
                ^ words[i - 15].rotate_right(18)
                ^ (words[i - 15] >> 3);
            let s1 = words[i - 2].rotate_right(17)
                ^ words[i - 2].rotate_right(19)
                ^ (words[i - 2] >> 10);
            words[i] = words[i - 16]
                .wrapping_add(s0)
                .wrapping_add(words[i - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;

        for (word, k) in words.iter().zip(Self::K) {
            let big_s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let temp1 = h
                .wrapping_add(big_s1)
                .wrapping_add(ch)
                .wrapping_add(k)
                .wrapping_add(*word);
            let big_s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = big_s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }
}

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut state = Sha256::new();
    state.update(data);
    state.finalize()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sha224 {
    state: Sha256,
}

impl Default for Sha224 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha224 {
    const INIT: [u32; 8] = [
        0xc105_9ed8,
        0x367c_d507,
        0x3070_dd17,
        0xf70e_5939,
        0xffc0_0b31,
        0x6858_1511,
        0x64f9_8fa7,
        0xbefa_4fa4,
    ];

    pub fn new() -> Self {
        Self {
            state: Sha256::with_init(Self::INIT),
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.state.update(data);
    }

    pub fn finalize(self) -> [u8; 28] {
        let words = self.state.finalize_words();
        let mut digest = [0_u8; 28];
        for (chunk, word) in digest.chunks_exact_mut(4).zip(words) {
            chunk.copy_from_slice(&word.to_be_bytes());
        }
        digest
    }
}

pub fn sha224(data: &[u8]) -> [u8; 28] {
    let mut state = Sha224::new();
    state.update(data);
    state.finalize()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sha512 {
    state: [u64; 8],
    len_bytes: u128,
    buffer: [u8; 128],
    buffer_len: usize,
}

impl Default for Sha512 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha512 {
    const INIT: [u64; 8] = [
        0x6a09_e667_f3bc_c908,
        0xbb67_ae85_84ca_a73b,
        0x3c6e_f372_fe94_f82b,
        0xa54f_f53a_5f1d_36f1,
        0x510e_527f_ade6_82d1,
        0x9b05_688c_2b3e_6c1f,
        0x1f83_d9ab_fb41_bd6b,
        0x5be0_cd19_137e_2179,
    ];
    const K: [u64; 80] = [
        0x428a_2f98_d728_ae22,
        0x7137_4491_23ef_65cd,
        0xb5c0_fbcf_ec4d_3b2f,
        0xe9b5_dba5_8189_dbbc,
        0x3956_c25b_f348_b538,
        0x59f1_11f1_b605_d019,
        0x923f_82a4_af19_4f9b,
        0xab1c_5ed5_da6d_8118,
        0xd807_aa98_a303_0242,
        0x1283_5b01_4570_6fbe,
        0x2431_85be_4ee4_b28c,
        0x550c_7dc3_d5ff_b4e2,
        0x72be_5d74_f27b_896f,
        0x80de_b1fe_3b16_96b1,
        0x9bdc_06a7_25c7_1235,
        0xc19b_f174_cf69_2694,
        0xe49b_69c1_9ef1_4ad2,
        0xefbe_4786_384f_25e3,
        0x0fc1_9dc6_8b8c_d5b5,
        0x240c_a1cc_77ac_9c65,
        0x2de9_2c6f_592b_0275,
        0x4a74_84aa_6ea6_e483,
        0x5cb0_a9dc_bd41_fbd4,
        0x76f9_88da_8311_53b5,
        0x983e_5152_ee66_dfab,
        0xa831_c66d_2db4_3210,
        0xb003_27c8_98fb_213f,
        0xbf59_7fc7_beef_0ee4,
        0xc6e0_0bf3_3da8_8fc2,
        0xd5a7_9147_930a_a725,
        0x06ca_6351_e003_826f,
        0x1429_2967_0a0e_6e70,
        0x27b7_0a85_46d2_2ffc,
        0x2e1b_2138_5c26_c926,
        0x4d2c_6dfc_5ac4_2aed,
        0x5338_0d13_9d95_b3df,
        0x650a_7354_8baf_63de,
        0x766a_0abb_3c77_b2a8,
        0x81c2_c92e_47ed_aee6,
        0x9272_2c85_1482_353b,
        0xa2bf_e8a1_4cf1_0364,
        0xa81a_664b_bc42_3001,
        0xc24b_8b70_d0f8_9791,
        0xc76c_51a3_0654_be30,
        0xd192_e819_d6ef_5218,
        0xd699_0624_5565_a910,
        0xf40e_3585_5771_202a,
        0x106a_a070_32bb_d1b8,
        0x19a4_c116_b8d2_d0c8,
        0x1e37_6c08_5141_ab53,
        0x2748_774c_df8e_eb99,
        0x34b0_bcb5_e19b_48a8,
        0x391c_0cb3_c5c9_5a63,
        0x4ed8_aa4a_e341_8acb,
        0x5b9c_ca4f_7763_e373,
        0x682e_6ff3_d6b2_b8a3,
        0x748f_82ee_5def_b2fc,
        0x78a5_636f_4317_2f60,
        0x84c8_7814_a1f0_ab72,
        0x8cc7_0208_1a64_39ec,
        0x90be_fffa_2363_1e28,
        0xa450_6ceb_de82_bde9,
        0xbef9_a3f7_b2c6_7915,
        0xc671_78f2_e372_532b,
        0xca27_3ece_ea26_619c,
        0xd186_b8c7_21c0_c207,
        0xeada_7dd6_cde0_eb1e,
        0xf57d_4f7f_ee6e_d178,
        0x06f0_67aa_7217_6fba,
        0x0a63_7dc5_a2c8_98a6,
        0x113f_9804_bef9_0dae,
        0x1b71_0b35_131c_471b,
        0x28db_77f5_2304_7d84,
        0x32ca_ab7b_40c7_2493,
        0x3c9e_be0a_15c9_bebc,
        0x431d_67c4_9c10_0d4c,
        0x4cc5_d4be_cb3e_42b6,
        0x597f_299c_fc65_7e2a,
        0x5fcb_6fab_3ad6_faec,
        0x6c44_198c_4a47_5817,
    ];

    pub fn new() -> Self {
        Self::with_init(Self::INIT)
    }

    fn with_init(init: [u64; 8]) -> Self {
        Self {
            state: init,
            len_bytes: 0,
            buffer: [0; 128],
            buffer_len: 0,
        }
    }

    pub fn update(&mut self, mut data: &[u8]) {
        self.len_bytes = self.len_bytes.wrapping_add(data.len() as u128);

        if self.buffer_len > 0 {
            let fill = (128 - self.buffer_len).min(data.len());
            self.buffer[self.buffer_len..self.buffer_len + fill].copy_from_slice(&data[..fill]);
            self.buffer_len += fill;
            data = &data[fill..];

            if self.buffer_len == 128 {
                let block = self.buffer;
                self.compress(&block);
                self.buffer_len = 0;
            }
        }

        let mut chunks = data.chunks_exact(128);
        for chunk in &mut chunks {
            let mut block = [0; 128];
            block.copy_from_slice(chunk);
            self.compress(&block);
        }

        let remainder = chunks.remainder();
        if !remainder.is_empty() {
            self.buffer[..remainder.len()].copy_from_slice(remainder);
            self.buffer_len = remainder.len();
        }
    }

    pub fn finalize(self) -> [u8; 64] {
        let words = self.finalize_words();
        let mut digest = [0_u8; 64];
        for (chunk, word) in digest.chunks_exact_mut(8).zip(words) {
            chunk.copy_from_slice(&word.to_be_bytes());
        }
        digest
    }

    fn finalize_words(mut self) -> [u64; 8] {
        let bit_len = self.len_bytes.wrapping_mul(8);
        let mut padding = [0_u8; 144];
        padding[0] = 0x80;
        let pad_len = if self.buffer_len < 112 {
            112 - self.buffer_len
        } else {
            240 - self.buffer_len
        };
        self.update(&padding[..pad_len]);
        self.update(&bit_len.to_be_bytes());

        self.state
    }

    fn compress(&mut self, block: &[u8; 128]) {
        let mut words = [0_u64; 80];
        for (word, chunk) in words.iter_mut().zip(block.chunks_exact(8)) {
            *word = u64::from_be_bytes([
                chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
            ]);
        }
        for i in 16..80 {
            let s0 = words[i - 15].rotate_right(1)
                ^ words[i - 15].rotate_right(8)
                ^ (words[i - 15] >> 7);
            let s1 =
                words[i - 2].rotate_right(19) ^ words[i - 2].rotate_right(61) ^ (words[i - 2] >> 6);
            words[i] = words[i - 16]
                .wrapping_add(s0)
                .wrapping_add(words[i - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;

        for (word, k) in words.iter().zip(Self::K) {
            let big_s1 = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
            let ch = (e & f) ^ (!e & g);
            let temp1 = h
                .wrapping_add(big_s1)
                .wrapping_add(ch)
                .wrapping_add(k)
                .wrapping_add(*word);
            let big_s0 = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = big_s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        self.state[0] = self.state[0].wrapping_add(a);
        self.state[1] = self.state[1].wrapping_add(b);
        self.state[2] = self.state[2].wrapping_add(c);
        self.state[3] = self.state[3].wrapping_add(d);
        self.state[4] = self.state[4].wrapping_add(e);
        self.state[5] = self.state[5].wrapping_add(f);
        self.state[6] = self.state[6].wrapping_add(g);
        self.state[7] = self.state[7].wrapping_add(h);
    }
}

pub fn sha512(data: &[u8]) -> [u8; 64] {
    let mut state = Sha512::new();
    state.update(data);
    state.finalize()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sha384 {
    state: Sha512,
}

impl Default for Sha384 {
    fn default() -> Self {
        Self::new()
    }
}

impl Sha384 {
    const INIT: [u64; 8] = [
        0xcbbb_9d5d_c105_9ed8,
        0x629a_292a_367c_d507,
        0x9159_015a_3070_dd17,
        0x152f_ecd8_f70e_5939,
        0x6733_2667_ffc0_0b31,
        0x8eb4_4a87_6858_1511,
        0xdb0c_2e0d_64f9_8fa7,
        0x47b5_481d_befa_4fa4,
    ];

    pub fn new() -> Self {
        Self {
            state: Sha512::with_init(Self::INIT),
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.state.update(data);
    }

    pub fn finalize(self) -> [u8; 48] {
        let words = self.state.finalize_words();
        let mut digest = [0_u8; 48];
        for (chunk, word) in digest.chunks_exact_mut(8).zip(words) {
            chunk.copy_from_slice(&word.to_be_bytes());
        }
        digest
    }
}

pub fn sha384(data: &[u8]) -> [u8; 48] {
    let mut state = Sha384::new();
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
    fn sha256_matches_standard_vectors() {
        assert_eq!(
            digest_to_hex(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            digest_to_hex(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            digest_to_hex(&sha256(
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
            )),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn sha256_streaming_matches_single_shot_across_block_boundaries() {
        let payload =
            b"abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnopqrstuvwxyz0123456789tail";
        let mut state = Sha256::new();

        state.update(&payload[..1]);
        state.update(&payload[1..55]);
        state.update(&payload[55..64]);
        state.update(&payload[64..]);

        assert_eq!(state.finalize(), sha256(payload));
    }

    #[test]
    fn sha224_matches_standard_vectors() {
        assert_eq!(
            digest_to_hex(&sha224(b"")),
            "d14a028c2a3a2bc9476102bb288234c415a2b01f828ea62ac5b3e42f"
        );
        assert_eq!(
            digest_to_hex(&sha224(b"abc")),
            "23097d223405d8228642a477bda255b32aadbce4bda0b3f7e36c9da7"
        );
        assert_eq!(
            digest_to_hex(&sha224(
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
            )),
            "75388b16512776cc5dba5da1fd890150b0c6455cb4f58b1952522525"
        );
    }

    #[test]
    fn sha224_streaming_matches_single_shot_across_block_boundaries() {
        let payload =
            b"abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnopqrstuvwxyz0123456789tail";
        let mut state = Sha224::new();

        state.update(&payload[..1]);
        state.update(&payload[1..55]);
        state.update(&payload[55..64]);
        state.update(&payload[64..]);

        assert_eq!(state.finalize(), sha224(payload));
    }

    #[test]
    fn sha512_matches_standard_vectors() {
        assert_eq!(
            digest_to_hex(&sha512(b"")),
            "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"
        );
        assert_eq!(
            digest_to_hex(&sha512(b"abc")),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
        assert_eq!(
            digest_to_hex(&sha512(
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
            )),
            "204a8fc6dda82f0a0ced7beb8e08a41657c16ef468b228a8279be331a703c33596fd15c13b1b07f9aa1d3bea57789ca031ad85c7a71dd70354ec631238ca3445"
        );
    }

    #[test]
    fn sha512_streaming_matches_single_shot_across_block_boundaries() {
        let payload = (0_u8..=199).collect::<Vec<_>>();
        let mut state = Sha512::new();

        state.update(&payload[..1]);
        state.update(&payload[1..111]);
        state.update(&payload[111..128]);
        state.update(&payload[128..]);

        assert_eq!(state.finalize(), sha512(&payload));
    }

    #[test]
    fn sha384_matches_standard_vectors() {
        assert_eq!(
            digest_to_hex(&sha384(b"")),
            "38b060a751ac96384cd9327eb1b1e36a21fdb71114be07434c0cc7bf63f6e1da274edebfe76f65fbd51ad2f14898b95b"
        );
        assert_eq!(
            digest_to_hex(&sha384(b"abc")),
            "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7"
        );
        assert_eq!(
            digest_to_hex(&sha384(
                b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
            )),
            "3391fdddfc8dc7393707a65b1b4709397cf8b1d162af05abfe8f450de5f36bc6b0455a8520bc4e6f5fe95b1fe3c8452b"
        );
    }

    #[test]
    fn sha384_streaming_matches_single_shot_across_block_boundaries() {
        let payload = (0_u8..=199).collect::<Vec<_>>();
        let mut state = Sha384::new();

        state.update(&payload[..1]);
        state.update(&payload[1..111]);
        state.update(&payload[111..128]);
        state.update(&payload[128..]);

        assert_eq!(state.finalize(), sha384(&payload));
    }

    #[test]
    fn digest_to_hex_formats_lowercase_bytes() {
        assert_eq!(digest_to_hex(&[0x00, 0x0f, 0x10, 0xff]), "000f10ff");
    }
}
