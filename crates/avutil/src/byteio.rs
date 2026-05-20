use crate::{AvError, AvErrorKind, AvResult};

#[derive(Debug, Clone)]
pub struct ByteReader<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> ByteReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn set_position(&mut self, position: usize) -> AvResult<()> {
        if position > self.data.len() {
            return Err(AvError::new(
                AvErrorKind::EndOfFile,
                format!(
                    "byte seek out of range: offset {position}, length {}",
                    self.data.len()
                ),
            ));
        }

        self.position = position;
        Ok(())
    }

    pub fn seek_relative(&mut self, offset: isize) -> AvResult<()> {
        let position = if offset >= 0 {
            self.position.checked_add(offset as usize).ok_or_else(|| {
                AvError::invalid_argument("byte seek offset overflows addressable memory")
            })?
        } else {
            self.position
                .checked_sub(offset.unsigned_abs())
                .ok_or_else(|| AvError::invalid_argument("byte seek before start of stream"))?
        };

        self.set_position(position)
    }

    pub fn rewind(&mut self) {
        self.position = 0;
    }

    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.position)
    }

    pub fn is_eof(&self) -> bool {
        self.remaining() == 0
    }

    pub fn skip(&mut self, count: usize) -> AvResult<()> {
        self.take(count).map(|_| ())
    }

    pub fn read_exact(&mut self, count: usize) -> AvResult<&'a [u8]> {
        self.take(count)
    }

    pub fn peek_exact(&self, count: usize) -> AvResult<&'a [u8]> {
        self.view(count)
    }

    pub fn read_u8(&mut self) -> AvResult<u8> {
        Ok(self.take(1)?[0])
    }

    pub fn peek_u8(&self) -> AvResult<u8> {
        Ok(self.view(1)?[0])
    }

    pub fn read_i8(&mut self) -> AvResult<i8> {
        Ok(i8::from_ne_bytes([self.read_u8()?]))
    }

    pub fn peek_i8(&self) -> AvResult<i8> {
        Ok(i8::from_ne_bytes([self.peek_u8()?]))
    }

    pub fn read_u16_le(&mut self) -> AvResult<u16> {
        let bytes = self.take(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn peek_u16_le(&self) -> AvResult<u16> {
        let bytes = self.view(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_u16_be(&mut self) -> AvResult<u16> {
        let bytes = self.take(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    pub fn peek_u16_be(&self) -> AvResult<u16> {
        let bytes = self.view(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_i16_le(&mut self) -> AvResult<i16> {
        let bytes = self.take(2)?;
        Ok(i16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn peek_i16_le(&self) -> AvResult<i16> {
        let bytes = self.view(2)?;
        Ok(i16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_i16_be(&mut self) -> AvResult<i16> {
        let bytes = self.take(2)?;
        Ok(i16::from_be_bytes([bytes[0], bytes[1]]))
    }

    pub fn peek_i16_be(&self) -> AvResult<i16> {
        let bytes = self.view(2)?;
        Ok(i16::from_be_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_u24_le(&mut self) -> AvResult<u32> {
        let bytes = self.take(3)?;
        Ok(u32::from(bytes[0]) | (u32::from(bytes[1]) << 8) | (u32::from(bytes[2]) << 16))
    }

    pub fn peek_u24_le(&self) -> AvResult<u32> {
        let bytes = self.view(3)?;
        Ok(u32::from(bytes[0]) | (u32::from(bytes[1]) << 8) | (u32::from(bytes[2]) << 16))
    }

    pub fn read_u24_be(&mut self) -> AvResult<u32> {
        let bytes = self.take(3)?;
        Ok((u32::from(bytes[0]) << 16) | (u32::from(bytes[1]) << 8) | u32::from(bytes[2]))
    }

    pub fn peek_u24_be(&self) -> AvResult<u32> {
        let bytes = self.view(3)?;
        Ok((u32::from(bytes[0]) << 16) | (u32::from(bytes[1]) << 8) | u32::from(bytes[2]))
    }

    pub fn read_i24_le(&mut self) -> AvResult<i32> {
        Ok(sign_extend_i24(self.read_u24_le()?))
    }

    pub fn peek_i24_le(&self) -> AvResult<i32> {
        Ok(sign_extend_i24(self.peek_u24_le()?))
    }

    pub fn read_i24_be(&mut self) -> AvResult<i32> {
        Ok(sign_extend_i24(self.read_u24_be()?))
    }

    pub fn peek_i24_be(&self) -> AvResult<i32> {
        Ok(sign_extend_i24(self.peek_u24_be()?))
    }

    pub fn read_u32_le(&mut self) -> AvResult<u32> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn peek_u32_le(&self) -> AvResult<u32> {
        let bytes = self.view(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn read_u32_be(&mut self) -> AvResult<u32> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn peek_u32_be(&self) -> AvResult<u32> {
        let bytes = self.view(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn read_u48_le(&mut self) -> AvResult<u64> {
        let bytes = self.take(6)?;
        Ok(u64::from(bytes[0])
            | (u64::from(bytes[1]) << 8)
            | (u64::from(bytes[2]) << 16)
            | (u64::from(bytes[3]) << 24)
            | (u64::from(bytes[4]) << 32)
            | (u64::from(bytes[5]) << 40))
    }

    pub fn peek_u48_le(&self) -> AvResult<u64> {
        let bytes = self.view(6)?;
        Ok(u64::from(bytes[0])
            | (u64::from(bytes[1]) << 8)
            | (u64::from(bytes[2]) << 16)
            | (u64::from(bytes[3]) << 24)
            | (u64::from(bytes[4]) << 32)
            | (u64::from(bytes[5]) << 40))
    }

    pub fn read_u48_be(&mut self) -> AvResult<u64> {
        let bytes = self.take(6)?;
        Ok((u64::from(bytes[0]) << 40)
            | (u64::from(bytes[1]) << 32)
            | (u64::from(bytes[2]) << 24)
            | (u64::from(bytes[3]) << 16)
            | (u64::from(bytes[4]) << 8)
            | u64::from(bytes[5]))
    }

    pub fn peek_u48_be(&self) -> AvResult<u64> {
        let bytes = self.view(6)?;
        Ok((u64::from(bytes[0]) << 40)
            | (u64::from(bytes[1]) << 32)
            | (u64::from(bytes[2]) << 24)
            | (u64::from(bytes[3]) << 16)
            | (u64::from(bytes[4]) << 8)
            | u64::from(bytes[5]))
    }

    pub fn read_i48_le(&mut self) -> AvResult<i64> {
        Ok(sign_extend_i48(self.read_u48_le()?))
    }

    pub fn peek_i48_le(&self) -> AvResult<i64> {
        Ok(sign_extend_i48(self.peek_u48_le()?))
    }

    pub fn read_i48_be(&mut self) -> AvResult<i64> {
        Ok(sign_extend_i48(self.read_u48_be()?))
    }

    pub fn peek_i48_be(&self) -> AvResult<i64> {
        Ok(sign_extend_i48(self.peek_u48_be()?))
    }

    pub fn read_i32_le(&mut self) -> AvResult<i32> {
        let bytes = self.take(4)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn peek_i32_le(&self) -> AvResult<i32> {
        let bytes = self.view(4)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn read_i32_be(&mut self) -> AvResult<i32> {
        let bytes = self.take(4)?;
        Ok(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn peek_i32_be(&self) -> AvResult<i32> {
        let bytes = self.view(4)?;
        Ok(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn read_u64_le(&mut self) -> AvResult<u64> {
        let bytes = self.take(8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    pub fn peek_u64_le(&self) -> AvResult<u64> {
        let bytes = self.view(8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    pub fn read_u64_be(&mut self) -> AvResult<u64> {
        let bytes = self.take(8)?;
        Ok(u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    pub fn peek_u64_be(&self) -> AvResult<u64> {
        let bytes = self.view(8)?;
        Ok(u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    pub fn read_i64_le(&mut self) -> AvResult<i64> {
        let bytes = self.take(8)?;
        Ok(i64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    pub fn peek_i64_le(&self) -> AvResult<i64> {
        let bytes = self.view(8)?;
        Ok(i64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    pub fn read_i64_be(&mut self) -> AvResult<i64> {
        let bytes = self.take(8)?;
        Ok(i64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    pub fn peek_i64_be(&self) -> AvResult<i64> {
        let bytes = self.view(8)?;
        Ok(i64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn take(&mut self, count: usize) -> AvResult<&'a [u8]> {
        let end = self.checked_end(count)?;
        let bytes = &self.data[self.position..end];
        self.position = end;
        Ok(bytes)
    }

    fn view(&self, count: usize) -> AvResult<&'a [u8]> {
        let end = self.checked_end(count)?;
        Ok(&self.data[self.position..end])
    }

    fn checked_end(&self, count: usize) -> AvResult<usize> {
        let end = self.position.checked_add(count).ok_or_else(|| {
            AvError::invalid_argument("byte read length overflows addressable memory")
        })?;

        if end > self.data.len() {
            return Err(AvError::new(
                AvErrorKind::EndOfFile,
                format!(
                    "unexpected end of byte stream: need {count} bytes at offset {}, {} remaining",
                    self.position,
                    self.remaining()
                ),
            ));
        }

        Ok(end)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ByteWriter {
    data: Vec<u8>,
}

impl ByteWriter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn position(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.data
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn truncate(&mut self, len: usize) -> AvResult<()> {
        if len > self.data.len() {
            return Err(AvError::new(
                AvErrorKind::EndOfFile,
                format!(
                    "byte truncate out of range: offset {len}, length {}",
                    self.data.len()
                ),
            ));
        }

        self.data.truncate(len);
        Ok(())
    }

    pub fn write_all(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes);
    }

    pub fn patch_all(&mut self, offset: usize, bytes: &[u8]) -> AvResult<()> {
        let end = self.checked_patch_end(offset, bytes.len())?;
        self.data[offset..end].copy_from_slice(bytes);
        Ok(())
    }

    pub fn write_u8(&mut self, value: u8) {
        self.data.push(value);
    }

    pub fn patch_u8(&mut self, offset: usize, value: u8) -> AvResult<()> {
        self.patch_all(offset, &[value])
    }

    pub fn write_i8(&mut self, value: i8) {
        self.write_all(&value.to_ne_bytes());
    }

    pub fn patch_i8(&mut self, offset: usize, value: i8) -> AvResult<()> {
        self.patch_all(offset, &value.to_ne_bytes())
    }

    pub fn write_u16_le(&mut self, value: u16) {
        self.write_all(&value.to_le_bytes());
    }

    pub fn patch_u16_le(&mut self, offset: usize, value: u16) -> AvResult<()> {
        self.patch_all(offset, &value.to_le_bytes())
    }

    pub fn write_u16_be(&mut self, value: u16) {
        self.write_all(&value.to_be_bytes());
    }

    pub fn patch_u16_be(&mut self, offset: usize, value: u16) -> AvResult<()> {
        self.patch_all(offset, &value.to_be_bytes())
    }

    pub fn write_i16_le(&mut self, value: i16) {
        self.write_all(&value.to_le_bytes());
    }

    pub fn patch_i16_le(&mut self, offset: usize, value: i16) -> AvResult<()> {
        self.patch_all(offset, &value.to_le_bytes())
    }

    pub fn write_i16_be(&mut self, value: i16) {
        self.write_all(&value.to_be_bytes());
    }

    pub fn patch_i16_be(&mut self, offset: usize, value: i16) -> AvResult<()> {
        self.patch_all(offset, &value.to_be_bytes())
    }

    pub fn write_u24_le(&mut self, value: u32) -> AvResult<()> {
        validate_u24(value)?;
        self.write_all(&[value as u8, (value >> 8) as u8, (value >> 16) as u8]);
        Ok(())
    }

    pub fn patch_u24_le(&mut self, offset: usize, value: u32) -> AvResult<()> {
        validate_u24(value)?;
        self.patch_all(
            offset,
            &[value as u8, (value >> 8) as u8, (value >> 16) as u8],
        )
    }

    pub fn write_u24_be(&mut self, value: u32) -> AvResult<()> {
        validate_u24(value)?;
        self.write_all(&[(value >> 16) as u8, (value >> 8) as u8, value as u8]);
        Ok(())
    }

    pub fn patch_u24_be(&mut self, offset: usize, value: u32) -> AvResult<()> {
        validate_u24(value)?;
        self.patch_all(
            offset,
            &[(value >> 16) as u8, (value >> 8) as u8, value as u8],
        )
    }

    pub fn write_i24_le(&mut self, value: i32) -> AvResult<()> {
        self.write_u24_le(encode_i24(value)?)
    }

    pub fn patch_i24_le(&mut self, offset: usize, value: i32) -> AvResult<()> {
        self.patch_u24_le(offset, encode_i24(value)?)
    }

    pub fn write_i24_be(&mut self, value: i32) -> AvResult<()> {
        self.write_u24_be(encode_i24(value)?)
    }

    pub fn patch_i24_be(&mut self, offset: usize, value: i32) -> AvResult<()> {
        self.patch_u24_be(offset, encode_i24(value)?)
    }

    pub fn write_u32_le(&mut self, value: u32) {
        self.write_all(&value.to_le_bytes());
    }

    pub fn patch_u32_le(&mut self, offset: usize, value: u32) -> AvResult<()> {
        self.patch_all(offset, &value.to_le_bytes())
    }

    pub fn write_u32_be(&mut self, value: u32) {
        self.write_all(&value.to_be_bytes());
    }

    pub fn patch_u32_be(&mut self, offset: usize, value: u32) -> AvResult<()> {
        self.patch_all(offset, &value.to_be_bytes())
    }

    pub fn write_u48_le(&mut self, value: u64) -> AvResult<()> {
        validate_u48(value)?;
        self.write_all(&[
            value as u8,
            (value >> 8) as u8,
            (value >> 16) as u8,
            (value >> 24) as u8,
            (value >> 32) as u8,
            (value >> 40) as u8,
        ]);
        Ok(())
    }

    pub fn patch_u48_le(&mut self, offset: usize, value: u64) -> AvResult<()> {
        validate_u48(value)?;
        self.patch_all(
            offset,
            &[
                value as u8,
                (value >> 8) as u8,
                (value >> 16) as u8,
                (value >> 24) as u8,
                (value >> 32) as u8,
                (value >> 40) as u8,
            ],
        )
    }

    pub fn write_u48_be(&mut self, value: u64) -> AvResult<()> {
        validate_u48(value)?;
        self.write_all(&[
            (value >> 40) as u8,
            (value >> 32) as u8,
            (value >> 24) as u8,
            (value >> 16) as u8,
            (value >> 8) as u8,
            value as u8,
        ]);
        Ok(())
    }

    pub fn patch_u48_be(&mut self, offset: usize, value: u64) -> AvResult<()> {
        validate_u48(value)?;
        self.patch_all(
            offset,
            &[
                (value >> 40) as u8,
                (value >> 32) as u8,
                (value >> 24) as u8,
                (value >> 16) as u8,
                (value >> 8) as u8,
                value as u8,
            ],
        )
    }

    pub fn write_i48_le(&mut self, value: i64) -> AvResult<()> {
        self.write_u48_le(encode_i48(value)?)
    }

    pub fn patch_i48_le(&mut self, offset: usize, value: i64) -> AvResult<()> {
        self.patch_u48_le(offset, encode_i48(value)?)
    }

    pub fn write_i48_be(&mut self, value: i64) -> AvResult<()> {
        self.write_u48_be(encode_i48(value)?)
    }

    pub fn patch_i48_be(&mut self, offset: usize, value: i64) -> AvResult<()> {
        self.patch_u48_be(offset, encode_i48(value)?)
    }

    pub fn write_i32_le(&mut self, value: i32) {
        self.write_all(&value.to_le_bytes());
    }

    pub fn patch_i32_le(&mut self, offset: usize, value: i32) -> AvResult<()> {
        self.patch_all(offset, &value.to_le_bytes())
    }

    pub fn write_i32_be(&mut self, value: i32) {
        self.write_all(&value.to_be_bytes());
    }

    pub fn patch_i32_be(&mut self, offset: usize, value: i32) -> AvResult<()> {
        self.patch_all(offset, &value.to_be_bytes())
    }

    pub fn write_u64_le(&mut self, value: u64) {
        self.write_all(&value.to_le_bytes());
    }

    pub fn patch_u64_le(&mut self, offset: usize, value: u64) -> AvResult<()> {
        self.patch_all(offset, &value.to_le_bytes())
    }

    pub fn write_u64_be(&mut self, value: u64) {
        self.write_all(&value.to_be_bytes());
    }

    pub fn patch_u64_be(&mut self, offset: usize, value: u64) -> AvResult<()> {
        self.patch_all(offset, &value.to_be_bytes())
    }

    pub fn write_i64_le(&mut self, value: i64) {
        self.write_all(&value.to_le_bytes());
    }

    pub fn patch_i64_le(&mut self, offset: usize, value: i64) -> AvResult<()> {
        self.patch_all(offset, &value.to_le_bytes())
    }

    pub fn write_i64_be(&mut self, value: i64) {
        self.write_all(&value.to_be_bytes());
    }

    pub fn patch_i64_be(&mut self, offset: usize, value: i64) -> AvResult<()> {
        self.patch_all(offset, &value.to_be_bytes())
    }

    fn checked_patch_end(&self, offset: usize, count: usize) -> AvResult<usize> {
        let end = offset.checked_add(count).ok_or_else(|| {
            AvError::invalid_argument("byte patch length overflows addressable memory")
        })?;

        if end > self.data.len() {
            return Err(AvError::new(
                AvErrorKind::EndOfFile,
                format!(
                    "byte patch out of range: need {count} bytes at offset {offset}, length {}",
                    self.data.len()
                ),
            ));
        }

        Ok(end)
    }
}

fn validate_u24(value: u32) -> AvResult<()> {
    if value > 0x00ff_ffff {
        return Err(AvError::invalid_argument("24-bit value out of range"));
    }
    Ok(())
}

fn validate_u48(value: u64) -> AvResult<()> {
    if value > 0x0000_ffff_ffff_ffff {
        return Err(AvError::invalid_argument("48-bit value out of range"));
    }
    Ok(())
}

fn sign_extend_i24(value: u32) -> i32 {
    ((value << 8) as i32) >> 8
}

fn sign_extend_i48(value: u64) -> i64 {
    ((value << 16) as i64) >> 16
}

fn encode_i24(value: i32) -> AvResult<u32> {
    const I24_MIN: i32 = -(1_i32 << 23);
    const I24_MAX: i32 = (1_i32 << 23) - 1;

    if !(I24_MIN..=I24_MAX).contains(&value) {
        return Err(AvError::invalid_argument(
            "signed 24-bit value out of range",
        ));
    }

    Ok((value as u32) & 0x00ff_ffff)
}

fn encode_i48(value: i64) -> AvResult<u64> {
    const I48_MIN: i64 = -(1_i64 << 47);
    const I48_MAX: i64 = (1_i64 << 47) - 1;

    if !(I48_MIN..=I48_MAX).contains(&value) {
        return Err(AvError::invalid_argument(
            "signed 48-bit value out of range",
        ));
    }

    Ok((value as u64) & 0x0000_ffff_ffff_ffff)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_unsigned_integers_in_both_endiannesses() {
        let mut reader = ByteReader::new(&[
            0x7f, 0x34, 0x12, 0x56, 0x78, 0xab, 0xcd, 0xef, 0x67, 0x45, 0x23, 0x01,
        ]);

        assert_eq!(reader.read_u8().unwrap(), 0x7f);
        assert_eq!(reader.read_u16_le().unwrap(), 0x1234);
        assert_eq!(reader.read_u16_be().unwrap(), 0x5678);
        assert_eq!(reader.read_u24_be().unwrap(), 0x00ab_cdef);
        assert_eq!(reader.read_u32_le().unwrap(), 0x0123_4567);
        assert!(reader.is_eof());
    }

    #[test]
    fn reads_signed_integers_in_both_endiannesses() {
        let mut reader = ByteReader::new(&[
            0xff, 0xff, 0x80, 0x00, 0x80, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xfe,
        ]);

        assert_eq!(reader.read_i16_le().unwrap(), -1);
        assert_eq!(reader.read_i16_be().unwrap(), -32768);
        assert_eq!(reader.read_i32_le().unwrap(), 128);
        assert_eq!(reader.read_i64_be().unwrap(), -2);
        assert!(reader.is_eof());
    }

    #[test]
    fn reads_u48_in_both_endiannesses() {
        let mut reader = ByteReader::new(&[
            0x06, 0x05, 0x04, 0x03, 0x02, 0x01, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06,
        ]);

        assert_eq!(reader.read_u48_le().unwrap(), 0x0102_0304_0506);
        assert_eq!(reader.read_u48_be().unwrap(), 0x0102_0304_0506);
        assert!(reader.is_eof());
    }

    #[test]
    fn reads_signed_24_and_48_bit_values_with_sign_extension() {
        let mut reader = ByteReader::new(&[
            0xfe, 0xff, 0xff, 0x80, 0x00, 0x00, 0xff, 0xff, 0x7f, 0xfe, 0xff, 0xff, 0xff, 0xff,
            0xff, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff,
        ]);

        assert_eq!(reader.read_i24_le().unwrap(), -2);
        assert_eq!(reader.read_i24_be().unwrap(), -8_388_608);
        assert_eq!(reader.read_i24_le().unwrap(), 8_388_607);
        assert_eq!(reader.read_i48_le().unwrap(), -2);
        assert_eq!(reader.read_i48_be().unwrap(), -140_737_488_355_328);
        assert_eq!(reader.read_i48_be().unwrap(), 140_737_488_355_327);
        assert!(reader.is_eof());
    }

    #[test]
    fn reports_typed_eof_without_advancing_past_end() {
        let mut reader = ByteReader::new(&[0x12, 0x34]);

        let err = reader.read_u32_be().unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::EndOfFile);
        assert_eq!(reader.position(), 0);
        assert_eq!(reader.remaining(), 2);
    }

    #[test]
    fn skip_and_exact_reads_advance_position() {
        let mut reader = ByteReader::new(&[0, 1, 2, 3, 4]);

        reader.skip(2).unwrap();
        assert_eq!(reader.position(), 2);
        assert_eq!(reader.read_exact(2).unwrap(), &[2, 3]);
        assert_eq!(reader.remaining(), 1);
    }

    #[test]
    fn set_position_seek_relative_and_rewind_are_checked() {
        let mut reader = ByteReader::new(&[10, 20, 30, 40]);

        reader.set_position(2).unwrap();
        assert_eq!(reader.read_u8().unwrap(), 30);

        reader.seek_relative(-2).unwrap();
        assert_eq!(reader.position(), 1);
        assert_eq!(reader.peek_u8().unwrap(), 20);

        reader.seek_relative(3).unwrap();
        assert_eq!(reader.position(), 4);
        assert!(reader.is_eof());

        let beyond = reader.seek_relative(1).unwrap_err();
        assert_eq!(beyond.kind(), AvErrorKind::EndOfFile);
        assert_eq!(reader.position(), 4);

        let before_start = reader.seek_relative(-5).unwrap_err();
        assert_eq!(before_start.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(reader.position(), 4);

        let absolute_beyond = reader.set_position(5).unwrap_err();
        assert_eq!(absolute_beyond.kind(), AvErrorKind::EndOfFile);
        assert_eq!(reader.position(), 4);

        reader.rewind();
        assert_eq!(reader.position(), 0);
        assert_eq!(reader.read_u8().unwrap(), 10);
    }

    #[test]
    fn peeks_unsigned_integers_without_advancing() {
        let reader = ByteReader::new(&[1, 2, 3, 4, 5, 6, 7, 8]);

        assert_eq!(reader.peek_exact(3).unwrap(), &[1, 2, 3]);
        assert_eq!(reader.peek_u8().unwrap(), 1);
        assert_eq!(reader.peek_u16_le().unwrap(), 0x0201);
        assert_eq!(reader.peek_u16_be().unwrap(), 0x0102);
        assert_eq!(reader.peek_u24_le().unwrap(), 0x030201);
        assert_eq!(reader.peek_u24_be().unwrap(), 0x010203);
        assert_eq!(reader.peek_u32_le().unwrap(), 0x0403_0201);
        assert_eq!(reader.peek_u32_be().unwrap(), 0x0102_0304);
        assert_eq!(reader.peek_u48_le().unwrap(), 0x0605_0403_0201);
        assert_eq!(reader.peek_u48_be().unwrap(), 0x0102_0304_0506);
        assert_eq!(reader.peek_u64_le().unwrap(), 0x0807_0605_0403_0201);
        assert_eq!(reader.peek_u64_be().unwrap(), 0x0102_0304_0506_0708);
        assert_eq!(reader.position(), 0);

        let mut reader = reader;
        assert_eq!(reader.read_exact(8).unwrap(), &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert!(reader.is_eof());
    }

    #[test]
    fn peeks_signed_integers_without_advancing() {
        let le = (-2_i64).to_le_bytes();
        let reader = ByteReader::new(&le);
        assert_eq!(reader.peek_i8().unwrap(), -2);
        assert_eq!(reader.peek_i16_le().unwrap(), -2);
        assert_eq!(reader.peek_i24_le().unwrap(), -2);
        assert_eq!(reader.peek_i32_le().unwrap(), -2);
        assert_eq!(reader.peek_i48_le().unwrap(), -2);
        assert_eq!(reader.peek_i64_le().unwrap(), -2);
        assert_eq!(reader.position(), 0);

        let reader = ByteReader::new(&[0xff, 0xfe]);
        assert_eq!(reader.peek_i16_be().unwrap(), -2);
        assert_eq!(reader.position(), 0);

        let reader = ByteReader::new(&[0xff, 0xff, 0xfe]);
        assert_eq!(reader.peek_i24_be().unwrap(), -2);
        assert_eq!(reader.position(), 0);

        let reader = ByteReader::new(&[0xff, 0xff, 0xff, 0xfe]);
        assert_eq!(reader.peek_i32_be().unwrap(), -2);
        assert_eq!(reader.position(), 0);

        let reader = ByteReader::new(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xfe]);
        assert_eq!(reader.peek_i48_be().unwrap(), -2);
        assert_eq!(reader.position(), 0);

        let be = (-2_i64).to_be_bytes();
        let reader = ByteReader::new(&be);
        assert_eq!(reader.peek_i64_be().unwrap(), -2);
        assert_eq!(reader.position(), 0);
    }

    #[test]
    fn peek_reports_typed_errors_without_advancing() {
        let mut reader = ByteReader::new(&[0x12, 0x34]);
        reader.skip(1).unwrap();

        let eof = reader.peek_u16_be().unwrap_err();
        assert_eq!(eof.kind(), AvErrorKind::EndOfFile);
        assert_eq!(reader.position(), 1);
        assert_eq!(reader.read_u8().unwrap(), 0x34);

        let overflow = reader.peek_exact(usize::MAX).unwrap_err();
        assert_eq!(overflow.kind(), AvErrorKind::InvalidArgument);
        assert!(reader.is_eof());
    }

    #[test]
    fn writes_unsigned_integers_in_both_endiannesses() {
        let mut writer = ByteWriter::new();

        writer.write_u8(0x7f);
        writer.write_u16_le(0x1234);
        writer.write_u16_be(0x5678);
        writer.write_u24_le(0x00ab_cdef).unwrap();
        writer.write_u24_be(0x0001_2345).unwrap();
        writer.write_u32_be(0x89ab_cdef);
        writer.write_u48_le(0x0102_0304_0506).unwrap();
        writer.write_u48_be(0x0a0b_0c0d_0e0f).unwrap();

        assert_eq!(
            writer.as_slice(),
            &[
                0x7f, 0x34, 0x12, 0x56, 0x78, 0xef, 0xcd, 0xab, 0x01, 0x23, 0x45, 0x89, 0xab, 0xcd,
                0xef, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
            ]
        );
    }

    #[test]
    fn writes_signed_integers_in_both_endiannesses() {
        let mut writer = ByteWriter::new();

        writer.write_i8(-1);
        writer.write_i16_le(-2);
        writer.write_i16_be(-3);
        writer.write_i24_le(-8_388_608).unwrap();
        writer.write_i24_be(8_388_607).unwrap();
        writer.write_i32_le(i32::MIN);
        writer.write_i32_be(i32::MAX);
        writer.write_i48_le(-140_737_488_355_328).unwrap();
        writer.write_i48_be(140_737_488_355_327).unwrap();
        writer.write_i64_le(i64::MIN);
        writer.write_i64_be(i64::MAX);

        let mut reader = ByteReader::new(writer.as_slice());
        assert_eq!(reader.read_i8().unwrap(), -1);
        assert_eq!(reader.read_i16_le().unwrap(), -2);
        assert_eq!(reader.read_i16_be().unwrap(), -3);
        assert_eq!(reader.read_i24_le().unwrap(), -8_388_608);
        assert_eq!(reader.read_i24_be().unwrap(), 8_388_607);
        assert_eq!(reader.read_i32_le().unwrap(), i32::MIN);
        assert_eq!(reader.read_i32_be().unwrap(), i32::MAX);
        assert_eq!(reader.read_i48_le().unwrap(), -140_737_488_355_328);
        assert_eq!(reader.read_i48_be().unwrap(), 140_737_488_355_327);
        assert_eq!(reader.read_i64_le().unwrap(), i64::MIN);
        assert_eq!(reader.read_i64_be().unwrap(), i64::MAX);
        assert!(reader.is_eof());
    }

    #[test]
    fn writer_rejects_values_too_wide_for_u24_and_u48() {
        let mut writer = ByteWriter::new();

        let u24_err = writer.write_u24_be(0x0100_0000).unwrap_err();
        let u48_err = writer.write_u48_be(0x0001_0000_0000_0000).unwrap_err();

        assert_eq!(u24_err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(u48_err.kind(), AvErrorKind::InvalidArgument);
        assert!(writer.is_empty());
    }

    #[test]
    fn writer_rejects_signed_values_too_wide_for_i24_and_i48_without_mutation() {
        let mut writer = ByteWriter::new();
        writer.write_u8(0xaa);

        let before = writer.as_slice().to_vec();
        let i24_high_err = writer.write_i24_le(8_388_608).unwrap_err();
        let i24_low_err = writer.write_i24_be(-8_388_609).unwrap_err();
        let i48_high_err = writer.write_i48_le(140_737_488_355_328).unwrap_err();
        let i48_low_err = writer.write_i48_be(-140_737_488_355_329).unwrap_err();

        assert_eq!(i24_high_err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(i24_low_err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(i48_high_err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(i48_low_err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(writer.as_slice(), before.as_slice());
    }

    #[test]
    fn writer_patches_existing_bytes_and_truncates() {
        let mut writer = ByteWriter::new();
        writer.write_all(&[0; 96]);

        writer.patch_u8(0, 0x7f).unwrap();
        writer.patch_i8(1, -1).unwrap();
        writer.patch_u16_le(2, 0x1234).unwrap();
        writer.patch_u16_be(4, 0x5678).unwrap();
        writer.patch_i16_le(6, -2).unwrap();
        writer.patch_i16_be(8, -3).unwrap();
        writer.patch_u24_le(10, 0x0001_0203).unwrap();
        writer.patch_u24_be(13, 0x00ab_cdef).unwrap();
        writer.patch_i24_le(16, -2).unwrap();
        writer.patch_i24_be(19, -8_388_608).unwrap();
        writer.patch_u32_le(22, 0x0506_0708).unwrap();
        writer.patch_u32_be(26, 0x0102_0304).unwrap();
        writer.patch_i32_le(30, i32::MIN).unwrap();
        writer.patch_i32_be(34, i32::MAX).unwrap();
        writer.patch_u48_le(38, 0x0102_0304_0506).unwrap();
        writer.patch_u48_be(44, 0x0a0b_0c0d_0e0f).unwrap();
        writer.patch_i48_le(50, -140_737_488_355_328).unwrap();
        writer.patch_i48_be(56, 140_737_488_355_327).unwrap();
        writer.patch_u64_le(62, 0x0102_0304_0506_0708).unwrap();
        writer.patch_u64_be(70, 0x1112_1314_1516_1718).unwrap();
        writer.patch_i64_le(78, i64::MIN).unwrap();
        writer.patch_i64_be(86, i64::MAX).unwrap();
        writer.patch_all(94, &[0xaa, 0xbb]).unwrap();

        assert_eq!(writer.position(), 96);
        let mut reader = ByteReader::new(writer.as_slice());
        assert_eq!(reader.read_u8().unwrap(), 0x7f);
        assert_eq!(reader.read_i8().unwrap(), -1);
        assert_eq!(reader.read_u16_le().unwrap(), 0x1234);
        assert_eq!(reader.read_u16_be().unwrap(), 0x5678);
        assert_eq!(reader.read_i16_le().unwrap(), -2);
        assert_eq!(reader.read_i16_be().unwrap(), -3);
        assert_eq!(reader.read_u24_le().unwrap(), 0x0001_0203);
        assert_eq!(reader.read_u24_be().unwrap(), 0x00ab_cdef);
        assert_eq!(reader.read_i24_le().unwrap(), -2);
        assert_eq!(reader.read_i24_be().unwrap(), -8_388_608);
        assert_eq!(reader.read_u32_le().unwrap(), 0x0506_0708);
        assert_eq!(reader.read_u32_be().unwrap(), 0x0102_0304);
        assert_eq!(reader.read_i32_le().unwrap(), i32::MIN);
        assert_eq!(reader.read_i32_be().unwrap(), i32::MAX);
        assert_eq!(reader.read_u48_le().unwrap(), 0x0102_0304_0506);
        assert_eq!(reader.read_u48_be().unwrap(), 0x0a0b_0c0d_0e0f);
        assert_eq!(reader.read_i48_le().unwrap(), -140_737_488_355_328);
        assert_eq!(reader.read_i48_be().unwrap(), 140_737_488_355_327);
        assert_eq!(reader.read_u64_le().unwrap(), 0x0102_0304_0506_0708);
        assert_eq!(reader.read_u64_be().unwrap(), 0x1112_1314_1516_1718);
        assert_eq!(reader.read_i64_le().unwrap(), i64::MIN);
        assert_eq!(reader.read_i64_be().unwrap(), i64::MAX);
        assert_eq!(reader.read_exact(2).unwrap(), &[0xaa, 0xbb]);

        writer.truncate(94).unwrap();
        assert_eq!(writer.len(), 94);
        writer.clear();
        assert!(writer.is_empty());
        assert_eq!(writer.position(), 0);
    }

    #[test]
    fn writer_patch_and_truncate_errors_preserve_buffer() {
        let mut writer = ByteWriter::new();
        writer.write_all(&[1, 2, 3, 4]);

        let before = writer.as_slice().to_vec();
        let patch_oob = writer.patch_u32_be(1, 0x0102_0304).unwrap_err();
        let patch_overflow = writer.patch_all(usize::MAX, &[0]).unwrap_err();
        let truncate_oob = writer.truncate(5).unwrap_err();
        let patch_u24_wide = writer.patch_u24_be(0, 0x0100_0000).unwrap_err();
        let patch_i48_wide = writer.patch_i48_le(0, 140_737_488_355_328).unwrap_err();

        assert_eq!(patch_oob.kind(), AvErrorKind::EndOfFile);
        assert_eq!(patch_overflow.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(truncate_oob.kind(), AvErrorKind::EndOfFile);
        assert_eq!(patch_u24_wide.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(patch_i48_wide.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(writer.as_slice(), before.as_slice());
    }
}
