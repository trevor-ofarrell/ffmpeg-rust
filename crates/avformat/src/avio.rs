use avutil::{AvError, AvErrorKind, AvResult};
use std::io::{self, Read, Seek, SeekFrom, Write};

#[derive(Debug, Clone)]
pub struct AvioReader<R> {
    inner: R,
}

impl<R> AvioReader<R>
where
    R: Read + Seek,
{
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    pub fn into_inner(self) -> R {
        self.inner
    }

    pub fn position(&mut self) -> AvResult<u64> {
        self.inner.stream_position().map_err(map_io_error)
    }

    pub fn seek(&mut self, position: u64) -> AvResult<u64> {
        self.inner
            .seek(SeekFrom::Start(position))
            .map_err(map_io_error)
    }

    pub fn skip(&mut self, offset: i64) -> AvResult<u64> {
        let current = self.position()?;
        let target = i128::from(current) + i128::from(offset);
        if target < 0 || target > i128::from(u64::MAX) {
            return Err(AvError::invalid_argument(
                "AVIO skip target is out of range",
            ));
        }

        self.seek(target as u64)
    }

    pub fn read_exact_vec(&mut self, count: usize) -> AvResult<Vec<u8>> {
        let start = self.position()?;
        let mut buffer = vec![0; count];
        match self.inner.read_exact(&mut buffer) {
            Ok(()) => Ok(buffer),
            Err(err) => {
                let _ = self.inner.seek(SeekFrom::Start(start));
                Err(map_io_error(err))
            }
        }
    }

    pub fn read_u8(&mut self) -> AvResult<u8> {
        Ok(self.read_exact_vec(1)?[0])
    }

    pub fn read_to_end(&mut self) -> AvResult<Vec<u8>> {
        let mut buffer = Vec::new();
        self.inner.read_to_end(&mut buffer).map_err(map_io_error)?;
        Ok(buffer)
    }
}

#[derive(Debug, Clone)]
pub struct AvioWriter<W> {
    inner: W,
}

impl<W> AvioWriter<W>
where
    W: Write + Seek,
{
    pub fn new(inner: W) -> Self {
        Self { inner }
    }

    pub fn into_inner(self) -> W {
        self.inner
    }

    pub fn position(&mut self) -> AvResult<u64> {
        self.inner.stream_position().map_err(map_io_error)
    }

    pub fn seek(&mut self, position: u64) -> AvResult<u64> {
        self.inner
            .seek(SeekFrom::Start(position))
            .map_err(map_io_error)
    }

    pub fn write_all(&mut self, bytes: &[u8]) -> AvResult<()> {
        self.inner.write_all(bytes).map_err(map_io_error)
    }

    pub fn write_u8(&mut self, value: u8) -> AvResult<()> {
        self.write_all(&[value])
    }

    pub fn flush(&mut self) -> AvResult<()> {
        self.inner.flush().map_err(map_io_error)
    }
}

fn map_io_error(err: io::Error) -> AvError {
    match err.kind() {
        io::ErrorKind::UnexpectedEof => {
            AvError::new(AvErrorKind::EndOfFile, "unexpected end of AVIO stream")
        }
        io::ErrorKind::InvalidInput => AvError::invalid_argument(err.to_string()),
        _ => AvError::new(AvErrorKind::External, err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn reader_reads_seeks_and_skips() {
        let mut reader = AvioReader::new(Cursor::new(vec![0, 1, 2, 3, 4, 5]));

        assert_eq!(reader.position().unwrap(), 0);
        assert_eq!(reader.read_u8().unwrap(), 0);
        assert_eq!(reader.skip(2).unwrap(), 3);
        assert_eq!(reader.read_exact_vec(2).unwrap(), vec![3, 4]);
        assert_eq!(reader.seek(1).unwrap(), 1);
        assert_eq!(reader.read_to_end().unwrap(), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn exact_read_eof_returns_typed_error_and_rewinds() {
        let mut reader = AvioReader::new(Cursor::new(vec![0xaa, 0xbb]));

        assert_eq!(reader.skip(1).unwrap(), 1);
        let err = reader.read_exact_vec(2).unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::EndOfFile);
        assert_eq!(reader.position().unwrap(), 1);
        assert_eq!(reader.read_u8().unwrap(), 0xbb);
    }

    #[test]
    fn skip_rejects_negative_positions() {
        let mut reader = AvioReader::new(Cursor::new(vec![1, 2, 3]));

        let err = reader.skip(-1).unwrap_err();

        assert_eq!(err.kind(), AvErrorKind::InvalidArgument);
        assert_eq!(reader.position().unwrap(), 0);
    }

    #[test]
    fn writer_writes_seeks_overwrites_and_flushes() {
        let mut writer = AvioWriter::new(Cursor::new(Vec::new()));

        writer.write_all(&[1, 2, 3]).unwrap();
        assert_eq!(writer.position().unwrap(), 3);
        writer.seek(1).unwrap();
        writer.write_u8(9).unwrap();
        writer.flush().unwrap();

        assert_eq!(writer.into_inner().into_inner(), vec![1, 9, 3]);
    }
}
