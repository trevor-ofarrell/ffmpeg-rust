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

    pub fn read_u8(&mut self) -> AvResult<u8> {
        Ok(self.take(1)?[0])
    }

    pub fn read_i8(&mut self) -> AvResult<i8> {
        Ok(i8::from_ne_bytes([self.read_u8()?]))
    }

    pub fn read_u16_le(&mut self) -> AvResult<u16> {
        let bytes = self.take(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_u16_be(&mut self) -> AvResult<u16> {
        let bytes = self.take(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_i16_le(&mut self) -> AvResult<i16> {
        let bytes = self.take(2)?;
        Ok(i16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_i16_be(&mut self) -> AvResult<i16> {
        let bytes = self.take(2)?;
        Ok(i16::from_be_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_u24_le(&mut self) -> AvResult<u32> {
        let bytes = self.take(3)?;
        Ok(u32::from(bytes[0]) | (u32::from(bytes[1]) << 8) | (u32::from(bytes[2]) << 16))
    }

    pub fn read_u24_be(&mut self) -> AvResult<u32> {
        let bytes = self.take(3)?;
        Ok((u32::from(bytes[0]) << 16) | (u32::from(bytes[1]) << 8) | u32::from(bytes[2]))
    }

    pub fn read_u32_le(&mut self) -> AvResult<u32> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn read_u32_be(&mut self) -> AvResult<u32> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn read_i32_le(&mut self) -> AvResult<i32> {
        let bytes = self.take(4)?;
        Ok(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn read_i32_be(&mut self) -> AvResult<i32> {
        let bytes = self.take(4)?;
        Ok(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn read_u64_le(&mut self) -> AvResult<u64> {
        let bytes = self.take(8)?;
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

    pub fn read_i64_le(&mut self) -> AvResult<i64> {
        let bytes = self.take(8)?;
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

    fn take(&mut self, count: usize) -> AvResult<&'a [u8]> {
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

        let bytes = &self.data[self.position..end];
        self.position = end;
        Ok(bytes)
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

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.data
    }

    pub fn write_all(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes);
    }

    pub fn write_u8(&mut self, value: u8) {
        self.data.push(value);
    }

    pub fn write_i8(&mut self, value: i8) {
        self.write_all(&value.to_ne_bytes());
    }

    pub fn write_u16_le(&mut self, value: u16) {
        self.write_all(&value.to_le_bytes());
    }

    pub fn write_u16_be(&mut self, value: u16) {
        self.write_all(&value.to_be_bytes());
    }

    pub fn write_i16_le(&mut self, value: i16) {
        self.write_all(&value.to_le_bytes());
    }

    pub fn write_i16_be(&mut self, value: i16) {
        self.write_all(&value.to_be_bytes());
    }

    pub fn write_u24_le(&mut self, value: u32) -> AvResult<()> {
        validate_u24(value)?;
        self.write_all(&[value as u8, (value >> 8) as u8, (value >> 16) as u8]);
        Ok(())
    }

    pub fn write_u24_be(&mut self, value: u32) -> AvResult<()> {
        validate_u24(value)?;
        self.write_all(&[(value >> 16) as u8, (value >> 8) as u8, value as u8]);
        Ok(())
    }

    pub fn write_u32_le(&mut self, value: u32) {
        self.write_all(&value.to_le_bytes());
    }

    pub fn write_u32_be(&mut self, value: u32) {
        self.write_all(&value.to_be_bytes());
    }

    pub fn write_i32_le(&mut self, value: i32) {
        self.write_all(&value.to_le_bytes());
    }

    pub fn write_i32_be(&mut self, value: i32) {
        self.write_all(&value.to_be_bytes());
    }

    pub fn write_u64_le(&mut self, value: u64) {
        self.write_all(&value.to_le_bytes());
    }

    pub fn write_u64_be(&mut self, value: u64) {
        self.write_all(&value.to_be_bytes());
    }

    pub fn write_i64_le(&mut self, value: i64) {
        self.write_all(&value.to_le_bytes());
    }

    pub fn write_i64_be(&mut self, value: i64) {
        self.write_all(&value.to_be_bytes());
    }
}

fn validate_u24(value: u32) -> AvResult<()> {
    if value > 0x00ff_ffff {
        return Err(AvError::invalid_argument("24-bit value out of range"));
    }
    Ok(())
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
    fn writes_unsigned_integers_in_both_endiannesses() {
        let mut writer = ByteWriter::new();

        writer.write_u8(0x7f);
        writer.write_u16_le(0x1234);
        writer.write_u16_be(0x5678);
        writer.write_u24_le(0x00ab_cdef).unwrap();
        writer.write_u24_be(0x0001_2345).unwrap();
        writer.write_u32_be(0x89ab_cdef);

        assert_eq!(
            writer.as_slice(),
            &[
                0x7f, 0x34, 0x12, 0x56, 0x78, 0xef, 0xcd, 0xab, 0x01, 0x23, 0x45, 0x89, 0xab, 0xcd,
                0xef,
            ]
        );
    }

    #[test]
    fn writer_rejects_values_too_wide_for_u24() {
        let mut writer = ByteWriter::new();

        let err = writer.write_u24_be(0x0100_0000).unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert!(writer.is_empty());
    }
}
