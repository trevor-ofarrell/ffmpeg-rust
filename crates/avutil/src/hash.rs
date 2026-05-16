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
}
