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

    pub fn set_bit_position(&mut self, bit_position: usize) -> AvResult<()> {
        if bit_position > self.len_bits() {
            return Err(AvError::new(
                AvErrorKind::EndOfFile,
                format!(
                    "bit seek out of range: bit offset {bit_position}, length {} bits",
                    self.len_bits()
                ),
            ));
        }

        self.bit_position = bit_position;
        Ok(())
    }

    pub fn seek_bits(&mut self, offset: isize) -> AvResult<()> {
        let bit_position = if offset >= 0 {
            self.bit_position
                .checked_add(offset as usize)
                .ok_or_else(|| {
                    AvError::invalid_argument("bit seek offset overflows addressable memory")
                })?
        } else {
            self.bit_position
                .checked_sub(offset.unsigned_abs())
                .ok_or_else(|| AvError::invalid_argument("bit seek before start of stream"))?
        };

        self.set_bit_position(bit_position)
    }

    pub fn rewind(&mut self) {
        self.bit_position = 0;
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

    pub fn peek_bit(&self) -> AvResult<bool> {
        Ok(self.peek_bits(1)? != 0)
    }

    pub fn read_aligned_bytes(&mut self, count: usize) -> AvResult<&'a [u8]> {
        let (start, end, requested_bits) = self.aligned_byte_range(count)?;
        self.bit_position += requested_bits;
        Ok(&self.data[start..end])
    }

    pub fn peek_aligned_bytes(&self, count: usize) -> AvResult<&'a [u8]> {
        let (start, end, _) = self.aligned_byte_range(count)?;
        Ok(&self.data[start..end])
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

    pub fn read_signed_bits(&mut self, count: u8) -> AvResult<i64> {
        self.read_bits(count).map(|value| sign_extend(value, count))
    }

    pub fn read_ue_golomb(&mut self) -> AvResult<u64> {
        let start = self.bit_position;
        match self.read_ue_golomb_inner() {
            Ok(value) => Ok(value),
            Err(err) => {
                self.bit_position = start;
                Err(err)
            }
        }
    }

    pub fn read_se_golomb(&mut self) -> AvResult<i64> {
        let start = self.bit_position;
        match self
            .read_ue_golomb_inner()
            .and_then(signed_exp_golomb_value)
        {
            Ok(value) => Ok(value),
            Err(err) => {
                self.bit_position = start;
                Err(err)
            }
        }
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

    pub fn peek_signed_bits(&self, count: u8) -> AvResult<i64> {
        self.peek_bits(count).map(|value| sign_extend(value, count))
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

    fn aligned_byte_range(&self, count: usize) -> AvResult<(usize, usize, usize)> {
        if !self.is_aligned() {
            return Err(AvError::invalid_argument(
                "cannot read whole bytes at unaligned bit offset",
            ));
        }

        let requested_bits = count
            .checked_mul(8)
            .ok_or_else(|| AvError::invalid_argument("byte read length overflows bit count"))?;

        if requested_bits > self.bits_remaining() {
            return Err(self.eof_error(requested_bits));
        }

        let start = self.bit_position / 8;
        let end = start.checked_add(count).ok_or_else(|| {
            AvError::invalid_argument("byte read end overflows addressable memory")
        })?;
        Ok((start, end, requested_bits))
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

    fn read_ue_golomb_inner(&mut self) -> AvResult<u64> {
        let mut leading_zero_bits = 0_usize;

        loop {
            if self.bits_remaining() == 0 {
                return Err(self.eof_error(1));
            }

            if self.read_bit()? {
                break;
            }

            leading_zero_bits += 1;
            if leading_zero_bits > 64 {
                return Err(AvError::invalid_data(
                    "unsigned Exp-Golomb code exceeds u64 range",
                ));
            }
        }

        if leading_zero_bits == 0 {
            return Ok(0);
        }

        let suffix = self.read_bits(leading_zero_bits as u8)?;
        let value = (1_u128 << leading_zero_bits) - 1 + u128::from(suffix);
        u64::try_from(value)
            .map_err(|_| AvError::invalid_data("unsigned Exp-Golomb code exceeds u64 range"))
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

fn signed_exp_golomb_value(code_num: u64) -> AvResult<i64> {
    if code_num == 0 {
        return Ok(0);
    }

    if code_num % 2 == 1 {
        let magnitude = code_num.div_ceil(2);
        i64::try_from(magnitude)
            .map_err(|_| AvError::invalid_data("signed Exp-Golomb value exceeds i64 range"))
    } else {
        let magnitude = code_num / 2;
        let magnitude = i64::try_from(magnitude)
            .map_err(|_| AvError::invalid_data("signed Exp-Golomb value exceeds i64 range"))?;
        Ok(-magnitude)
    }
}

fn sign_extend(value: u64, count: u8) -> i64 {
    if count == 0 {
        0
    } else if count == 64 {
        value as i64
    } else {
        let sign_bit = 1_u64 << (count - 1);
        if value & sign_bit == 0 {
            value as i64
        } else {
            let sign_mask = !0_u64 << count;
            (value | sign_mask) as i64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes_from_bits(bits: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut current = 0_u8;
        for (index, bit) in bits.bytes().enumerate() {
            current <<= 1;
            if bit == b'1' {
                current |= 1;
            }
            if index % 8 == 7 {
                bytes.push(current);
                current = 0;
            }
        }
        let remainder = bits.len() % 8;
        if remainder != 0 {
            current <<= 8 - remainder;
            bytes.push(current);
        }
        bytes
    }

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

        assert!(reader.peek_bit().unwrap());
        assert_eq!(reader.bit_position(), 0);
        assert_eq!(reader.peek_bits(4).unwrap(), 0b1100);
        assert_eq!(reader.bit_position(), 0);
        assert!(reader.read_bit().unwrap());
        assert_eq!(reader.bit_position(), 1);
        assert!(reader.peek_bit().unwrap());
        assert_eq!(reader.bit_position(), 1);
        reader.rewind();
        assert_eq!(reader.read_bits(4).unwrap(), 0b1100);
        assert_eq!(reader.bit_position(), 4);
    }

    #[test]
    fn aligned_bytes_read_and_peek_without_bit_iteration() {
        let mut reader = BitReader::new(&[0xab, 0xcd, 0xef]);

        assert_eq!(reader.peek_aligned_bytes(2).unwrap(), &[0xab, 0xcd]);
        assert_eq!(reader.bit_position(), 0);
        assert_eq!(reader.read_aligned_bytes(1).unwrap(), &[0xab]);
        assert_eq!(reader.bit_position(), 8);
        assert_eq!(reader.peek_aligned_bytes(2).unwrap(), &[0xcd, 0xef]);
        assert_eq!(reader.bit_position(), 8);
        assert_eq!(reader.read_aligned_bytes(2).unwrap(), &[0xcd, 0xef]);
        assert!(reader.is_eof());

        assert_eq!(reader.read_aligned_bytes(0).unwrap(), &[]);
        assert_eq!(reader.bit_position(), 24);
    }

    #[test]
    fn aligned_bytes_errors_do_not_advance_cursor() {
        let mut unaligned = BitReader::new(&[0xff, 0x00]);
        unaligned.skip_bits(3).unwrap();

        let err = unaligned.read_aligned_bytes(1).unwrap_err();
        let peek_err = unaligned.peek_aligned_bytes(1).unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(peek_err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(unaligned.bit_position(), 3);

        let mut short = BitReader::new(&[0xab]);
        short.skip_bits(8).unwrap();
        let eof = short.read_aligned_bytes(1).unwrap_err();
        let peek_eof = short.peek_aligned_bytes(1).unwrap_err();

        assert_eq!(eof.kind(), AvErrorKind::EndOfFile);
        assert_eq!(peek_eof.kind(), AvErrorKind::EndOfFile);
        assert_eq!(short.bit_position(), 8);

        let overflow = short.peek_aligned_bytes(usize::MAX).unwrap_err();
        assert_eq!(overflow.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(short.bit_position(), 8);
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
    fn set_position_seek_bits_and_rewind_are_checked() {
        let mut reader = BitReader::new(&[0b1011_0010, 0b0110_0001]);

        reader.set_bit_position(5).unwrap();
        assert_eq!(reader.read_bits(4).unwrap(), 0b0100);

        reader.seek_bits(-6).unwrap();
        assert_eq!(reader.bit_position(), 3);
        assert_eq!(reader.peek_bits(5).unwrap(), 0b1_0010);

        reader.seek_bits(13).unwrap();
        assert_eq!(reader.bit_position(), 16);
        assert!(reader.is_eof());

        let beyond = reader.seek_bits(1).unwrap_err();
        assert_eq!(beyond.kind(), AvErrorKind::EndOfFile);
        assert_eq!(reader.bit_position(), 16);

        let before_start = reader.seek_bits(-17).unwrap_err();
        assert_eq!(before_start.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(reader.bit_position(), 16);

        let absolute_beyond = reader.set_bit_position(17).unwrap_err();
        assert_eq!(absolute_beyond.kind(), AvErrorKind::EndOfFile);
        assert_eq!(reader.bit_position(), 16);

        reader.rewind();
        assert_eq!(reader.bit_position(), 0);
        assert_eq!(reader.read_bits(3).unwrap(), 0b101);
    }

    #[test]
    fn zero_bit_reads_are_valid_noops() {
        let mut reader = BitReader::new(&[0xff]);

        assert_eq!(reader.read_bits(0).unwrap(), 0);
        assert_eq!(reader.peek_bits(0).unwrap(), 0);
        assert_eq!(reader.read_signed_bits(0).unwrap(), 0);
        assert_eq!(reader.peek_signed_bits(0).unwrap(), 0);
        assert_eq!(reader.bit_position(), 0);
    }

    #[test]
    fn signed_reads_sign_extend_and_peek_without_advancing() {
        let mut reader =
            BitReader::new(&[0b1110_0101, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

        assert_eq!(reader.peek_signed_bits(4).unwrap(), -2);
        assert_eq!(reader.bit_position(), 0);
        assert_eq!(reader.read_signed_bits(4).unwrap(), -2);
        assert_eq!(reader.read_signed_bits(4).unwrap(), 5);
        assert_eq!(reader.read_signed_bits(1).unwrap(), -1);
        assert_eq!(reader.read_signed_bits(63).unwrap(), 0);
        assert!(reader.is_eof());
    }

    #[test]
    fn reads_unsigned_exp_golomb_codes() {
        let bytes = bytes_from_bits("101001100100001010011000111");
        let mut reader = BitReader::new(&bytes);

        for expected in 0..=6 {
            assert_eq!(reader.read_ue_golomb().unwrap(), expected);
        }
        assert_eq!(reader.bit_position(), 27);
    }

    #[test]
    fn reads_signed_exp_golomb_codes() {
        let bytes = bytes_from_bits("101001100100001010011000111");
        let mut reader = BitReader::new(&bytes);

        for expected in [0, 1, -1, 2, -2, 3, -3] {
            assert_eq!(reader.read_se_golomb().unwrap(), expected);
        }
        assert_eq!(reader.bit_position(), 27);
    }

    #[test]
    fn eof_errors_do_not_advance_cursor() {
        let mut reader = BitReader::new(&[0b1000_0000]);

        reader.skip_bits(7).unwrap();
        let err = reader.read_bits(2).unwrap_err();
        let signed_err = reader.read_signed_bits(2).unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::EndOfFile);
        assert_eq!(signed_err.kind(), AvErrorKind::EndOfFile);
        assert_eq!(reader.bit_position(), 7);
        assert_eq!(reader.bits_remaining(), 1);
    }

    #[test]
    fn exp_golomb_errors_do_not_advance_cursor() {
        let mut truncated = BitReader::new(&[0]);
        let eof = truncated.read_ue_golomb().unwrap_err();
        assert_eq!(eof.kind(), AvErrorKind::EndOfFile);
        assert_eq!(truncated.bit_position(), 0);

        let oversized = bytes_from_bits(&format!("{}1", "0".repeat(65)));
        let mut oversized_reader = BitReader::new(&oversized);
        let err = oversized_reader.read_ue_golomb().unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);
        assert_eq!(oversized_reader.bit_position(), 0);

        let signed_too_large = bytes_from_bits(&format!("{}1{}", "0".repeat(64), "0".repeat(64)));
        let mut signed_reader = BitReader::new(&signed_too_large);
        let err = signed_reader.read_se_golomb().unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);
        assert_eq!(signed_reader.bit_position(), 0);
    }

    #[test]
    fn rejects_reads_wider_than_u64() {
        let mut reader = BitReader::new(&[0xff; 9]);

        let err = reader.read_bits(65).unwrap_err();
        let signed_err = reader.read_signed_bits(65).unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(signed_err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(reader.bit_position(), 0);
    }
}
