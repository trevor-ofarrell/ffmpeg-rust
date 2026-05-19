use crate::{AvError, AvResult};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BitWriter {
    data: Vec<u8>,
    bit_position: usize,
}

impl BitWriter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
            bit_position: 0,
        }
    }

    pub fn bit_position(&self) -> usize {
        self.bit_position
    }

    pub fn bits_written(&self) -> usize {
        self.bit_position
    }

    pub fn is_empty(&self) -> bool {
        self.bit_position == 0
    }

    pub fn is_aligned(&self) -> bool {
        self.bit_position % 8 == 0
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.data
    }

    pub fn write_bit(&mut self, value: bool) {
        if self.is_aligned() {
            self.data.push(0);
        }

        if value {
            let byte_index = self.bit_position / 8;
            let shift = 7 - (self.bit_position % 8);
            self.data[byte_index] |= 1_u8 << shift;
        }

        self.bit_position += 1;
    }

    pub fn write_bits(&mut self, value: u64, count: u8) -> AvResult<()> {
        validate_width(value, count)?;

        for shift in (0..count).rev() {
            self.write_bit(((value >> shift) & 1) != 0);
        }

        Ok(())
    }

    pub fn write_signed_bits(&mut self, value: i64, count: u8) -> AvResult<()> {
        let encoded = validate_signed_width(value, count)?;
        self.write_bits(encoded, count)
    }

    pub fn write_ue_golomb(&mut self, value: u64) -> AvResult<()> {
        let code_num = u128::from(value) + 1;
        let leading_zero_bits = 127 - code_num.leading_zeros() as usize;
        let suffix = code_num - (1_u128 << leading_zero_bits);

        for _ in 0..leading_zero_bits {
            self.write_bit(false);
        }
        self.write_bit(true);
        if leading_zero_bits > 0 {
            self.write_bits(suffix as u64, leading_zero_bits as u8)?;
        }
        Ok(())
    }

    pub fn write_se_golomb(&mut self, value: i64) -> AvResult<()> {
        let code_num = signed_exp_golomb_code_num(value)?;
        self.write_ue_golomb(code_num)
    }

    pub fn byte_align_zero(&mut self) {
        while !self.is_aligned() {
            self.write_bit(false);
        }
    }
}

fn validate_width(value: u64, count: u8) -> AvResult<()> {
    if count > 64 {
        return Err(AvError::invalid_argument(
            "cannot write more than 64 bits at once",
        ));
    }

    if count == 0 {
        if value == 0 {
            return Ok(());
        }
        return Err(AvError::invalid_argument(
            "non-zero value does not fit in zero bits",
        ));
    }

    if count < 64 && value >= (1_u64 << count) {
        return Err(AvError::invalid_argument(
            "bit value does not fit requested width",
        ));
    }

    Ok(())
}

fn validate_signed_width(value: i64, count: u8) -> AvResult<u64> {
    if count > 64 {
        return Err(AvError::invalid_argument(
            "cannot write more than 64 signed bits at once",
        ));
    }

    if count == 0 {
        if value == 0 {
            return Ok(0);
        }
        return Err(AvError::invalid_argument(
            "non-zero signed value does not fit in zero bits",
        ));
    }

    if count == 64 {
        return Ok(value as u64);
    }

    let shift = count - 1;
    let min = -(1_i64 << shift);
    let max = (1_i64 << shift) - 1;
    if value < min || value > max {
        return Err(AvError::invalid_argument(
            "signed bit value does not fit requested width",
        ));
    }

    Ok((value as u64) & ((1_u64 << count) - 1))
}

fn signed_exp_golomb_code_num(value: i64) -> AvResult<u64> {
    if value > 0 {
        Ok((value as u64) * 2 - 1)
    } else if value == 0 {
        Ok(0)
    } else {
        let magnitude = value
            .checked_neg()
            .ok_or_else(|| AvError::invalid_argument("i64::MIN cannot be Exp-Golomb encoded"))?;
        Ok((magnitude as u64) * 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AvErrorKind, BitReader};

    fn bits_from_bytes(bytes: &[u8], bit_len: usize) -> String {
        let mut bits = String::with_capacity(bit_len);
        for position in 0..bit_len {
            let byte = bytes[position / 8];
            let shift = 7 - (position % 8);
            if (byte >> shift) & 1 == 1 {
                bits.push('1');
            } else {
                bits.push('0');
            }
        }
        bits
    }

    #[test]
    fn writes_msb_first_bits_across_byte_boundaries() {
        let mut writer = BitWriter::new();

        writer.write_bits(0b101, 3).unwrap();
        writer.write_bit(true);
        writer.write_bits(0b0010, 4).unwrap();
        writer.write_bits(0b0110_0001, 8).unwrap();

        assert_eq!(writer.as_slice(), &[0b1011_0010, 0b0110_0001]);
        assert_eq!(writer.bits_written(), 16);
        assert!(writer.is_aligned());
    }

    #[test]
    fn round_trips_with_bitreader() {
        let mut writer = BitWriter::new();
        writer.write_bits(0xabc, 12).unwrap();
        writer.write_bits(0x5, 3).unwrap();
        writer.write_bit(false);

        let bytes = writer.into_inner();
        let mut reader = BitReader::new(&bytes);

        assert_eq!(reader.read_bits(12).unwrap(), 0xabc);
        assert_eq!(reader.read_bits(3).unwrap(), 0x5);
        assert!(!reader.read_bit().unwrap());
        assert!(reader.is_eof());
    }

    #[test]
    fn signed_bits_round_trip_with_sign_extension() {
        let mut writer = BitWriter::new();
        writer.write_signed_bits(-2, 4).unwrap();
        writer.write_signed_bits(5, 4).unwrap();
        writer.write_signed_bits(-1, 1).unwrap();
        writer.write_signed_bits(i64::MIN, 64).unwrap();
        writer.byte_align_zero();

        let bytes = writer.into_inner();
        let mut reader = BitReader::new(&bytes);

        assert_eq!(reader.read_signed_bits(4).unwrap(), -2);
        assert_eq!(reader.read_signed_bits(4).unwrap(), 5);
        assert_eq!(reader.read_signed_bits(1).unwrap(), -1);
        assert_eq!(reader.read_signed_bits(64).unwrap(), i64::MIN);
        assert_eq!(reader.read_bits(7).unwrap(), 0);
        assert!(reader.is_eof());
    }

    #[test]
    fn writes_unsigned_exp_golomb_codes() {
        let mut writer = BitWriter::new();

        for value in 0..=6 {
            writer.write_ue_golomb(value).unwrap();
        }

        assert_eq!(
            bits_from_bytes(writer.as_slice(), writer.bits_written()),
            "101001100100001010011000111"
        );

        let bytes = writer.into_inner();
        let mut reader = BitReader::new(&bytes);
        for expected in 0..=6 {
            assert_eq!(reader.read_ue_golomb().unwrap(), expected);
        }
    }

    #[test]
    fn writes_signed_exp_golomb_codes() {
        let mut writer = BitWriter::new();

        for value in [0, 1, -1, 2, -2, 3, -3] {
            writer.write_se_golomb(value).unwrap();
        }

        assert_eq!(
            bits_from_bytes(writer.as_slice(), writer.bits_written()),
            "101001100100001010011000111"
        );

        let bytes = writer.into_inner();
        let mut reader = BitReader::new(&bytes);
        for expected in [0, 1, -1, 2, -2, 3, -3] {
            assert_eq!(reader.read_se_golomb().unwrap(), expected);
        }
    }

    #[test]
    fn writes_u64_max_unsigned_exp_golomb() {
        let mut writer = BitWriter::new();

        writer.write_ue_golomb(u64::MAX).unwrap();

        assert_eq!(writer.bits_written(), 129);
        let bytes = writer.into_inner();
        let mut reader = BitReader::new(&bytes);
        assert_eq!(reader.read_ue_golomb().unwrap(), u64::MAX);
    }

    #[test]
    fn byte_align_zero_pads_partial_byte() {
        let mut writer = BitWriter::new();

        writer.write_bits(0b101, 3).unwrap();
        writer.byte_align_zero();

        assert_eq!(writer.as_slice(), &[0b1010_0000]);
        assert_eq!(writer.bits_written(), 8);
        assert!(writer.is_aligned());
    }

    #[test]
    fn zero_bit_zero_value_write_is_noop() {
        let mut writer = BitWriter::new();

        writer.write_bits(0, 0).unwrap();
        writer.write_signed_bits(0, 0).unwrap();

        assert!(writer.is_empty());
        assert!(writer.as_slice().is_empty());
    }

    #[test]
    fn rejects_values_that_do_not_fit_width_without_advancing() {
        let mut writer = BitWriter::new();

        let err = writer.write_bits(0b100, 2).unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(writer.bit_position(), 0);
        assert!(writer.as_slice().is_empty());
    }

    #[test]
    fn rejects_nonzero_zero_width_write() {
        let mut writer = BitWriter::new();

        let err = writer.write_bits(1, 0).unwrap_err();
        let signed_err = writer.write_signed_bits(1, 0).unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(signed_err.kind(), AvErrorKind::InvalidArgument);
        assert!(writer.is_empty());
    }

    #[test]
    fn rejects_widths_wider_than_u64() {
        let mut writer = BitWriter::new();

        let err = writer.write_bits(0, 65).unwrap_err();
        let signed_err = writer.write_signed_bits(0, 65).unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(signed_err.kind(), AvErrorKind::InvalidArgument);
        assert!(writer.is_empty());
    }

    #[test]
    fn rejects_signed_values_that_do_not_fit_width_without_advancing() {
        let mut writer = BitWriter::new();

        let positive_err = writer.write_signed_bits(2, 2).unwrap_err();
        let negative_err = writer.write_signed_bits(-3, 2).unwrap_err();

        assert_eq!(positive_err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(negative_err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(writer.bit_position(), 0);
        assert!(writer.as_slice().is_empty());
    }

    #[test]
    fn rejects_unrepresentable_signed_exp_golomb_without_advancing() {
        let mut writer = BitWriter::new();

        let err = writer.write_se_golomb(i64::MIN).unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(writer.bit_position(), 0);
        assert!(writer.as_slice().is_empty());
    }
}
