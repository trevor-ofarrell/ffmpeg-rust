use crate::{AvError, AvErrorKind, AvResult};

#[derive(Debug, Clone)]
pub struct BitReader<'a> {
    data: &'a [u8],
    bit_position: usize,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            bit_position: 0,
        }
    }

    pub fn len_bits(&self) -> usize {
        self.data.len().saturating_mul(8)
    }

    pub fn bit_position(&self) -> usize {
        self.bit_position
    }

    pub fn bits_remaining(&self) -> usize {
        self.len_bits().saturating_sub(self.bit_position)
    }

    pub fn is_aligned(&self) -> bool {
        self.bit_position % 8 == 0
    }

    pub fn is_eof(&self) -> bool {
        self.bits_remaining() == 0
    }

    pub fn read_bit(&mut self) -> AvResult<bool> {
        Ok(self.read_bits(1)? != 0)
    }

    pub fn read_bits(&mut self, count: u8) -> AvResult<u64> {
        self.validate_read(count)?;

        let mut value = 0_u64;
        for _ in 0..count {
            value = (value << 1) | u64::from(self.bit_at(self.bit_position));
            self.bit_position += 1;
        }
        Ok(value)
    }

    pub fn peek_bits(&self, count: u8) -> AvResult<u64> {
        self.validate_read(count)?;

        let mut value = 0_u64;
        for bit_index in 0..usize::from(count) {
            let position = self.bit_position + bit_index;
            value = (value << 1) | u64::from(self.bit_at(position));
        }
        Ok(value)
    }

    pub fn skip_bits(&mut self, count: usize) -> AvResult<()> {
        let end = self.bit_position.checked_add(count).ok_or_else(|| {
            AvError::invalid_argument("bit skip length overflows addressable memory")
        })?;

        if end > self.len_bits() {
            return Err(self.eof_error(count));
        }

        self.bit_position = end;
        Ok(())
    }

    pub fn byte_align(&mut self) -> AvResult<()> {
        let extra = self.bit_position % 8;
        if extra == 0 {
            return Ok(());
        }
        self.skip_bits(8 - extra)
    }

    fn validate_read(&self, count: u8) -> AvResult<()> {
        if count > 64 {
            return Err(AvError::invalid_argument(
                "cannot read more than 64 bits at once",
            ));
        }

        if usize::from(count) > self.bits_remaining() {
            return Err(self.eof_error(usize::from(count)));
        }

        Ok(())
    }

    fn eof_error(&self, requested: usize) -> AvError {
        AvError::new(
            AvErrorKind::EndOfFile,
            format!(
                "unexpected end of bit stream: need {requested} bits at bit offset {}, {} remaining",
                self.bit_position,
                self.bits_remaining()
            ),
        )
    }

    fn bit_at(&self, position: usize) -> u8 {
        let byte = self.data[position / 8];
        let shift = 7 - (position % 8);
        (byte >> shift) & 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_msb_first_bits_across_byte_boundaries() {
        let mut reader = BitReader::new(&[0b1011_0010, 0b0110_0001]);

        assert_eq!(reader.read_bits(3).unwrap(), 0b101);
        assert_eq!(reader.read_bits(5).unwrap(), 0b1_0010);
        assert_eq!(reader.read_bits(4).unwrap(), 0b0110);
        assert_eq!(reader.read_bits(4).unwrap(), 0b0001);
        assert!(reader.is_eof());
    }

    #[test]
    fn peek_does_not_advance_cursor() {
        let mut reader = BitReader::new(&[0b1100_0000]);

        assert_eq!(reader.peek_bits(4).unwrap(), 0b1100);
        assert_eq!(reader.bit_position(), 0);
        assert_eq!(reader.read_bits(4).unwrap(), 0b1100);
        assert_eq!(reader.bit_position(), 4);
    }

    #[test]
    fn skip_and_align_update_cursor() {
        let mut reader = BitReader::new(&[0xff, 0x80]);

        reader.skip_bits(3).unwrap();
        assert!(!reader.is_aligned());
        reader.byte_align().unwrap();
        assert!(reader.is_aligned());
        assert_eq!(reader.bit_position(), 8);
        assert!(reader.read_bit().unwrap());
    }

    #[test]
    fn zero_bit_reads_are_valid_noops() {
        let mut reader = BitReader::new(&[0xff]);

        assert_eq!(reader.read_bits(0).unwrap(), 0);
        assert_eq!(reader.peek_bits(0).unwrap(), 0);
        assert_eq!(reader.bit_position(), 0);
    }

    #[test]
    fn eof_errors_do_not_advance_cursor() {
        let mut reader = BitReader::new(&[0b1000_0000]);

        reader.skip_bits(7).unwrap();
        let err = reader.read_bits(2).unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::EndOfFile);
        assert_eq!(reader.bit_position(), 7);
        assert_eq!(reader.bits_remaining(), 1);
    }

    #[test]
    fn rejects_reads_wider_than_u64() {
        let mut reader = BitReader::new(&[0xff; 9]);

        let err = reader.read_bits(65).unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(reader.bit_position(), 0);
    }
}
