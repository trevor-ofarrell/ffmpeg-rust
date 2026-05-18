use crate::{AvError, AvResult};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferRef {
    data: Arc<Vec<u8>>,
}

impl BufferRef {
    pub fn from_vec(data: Vec<u8>) -> Self {
        Self {
            data: Arc::new(data),
        }
    }

    pub fn copy_from_slice(data: &[u8]) -> Self {
        Self::from_vec(data.to_vec())
    }

    pub fn zeroed(size: usize) -> AvResult<Self> {
        let mut data = Vec::new();
        data.try_reserve_exact(size)
            .map_err(|_| AvError::external(format!("failed to allocate {size} buffer bytes")))?;
        data.resize(size, 0);
        Ok(Self::from_vec(data))
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn as_slice(&self) -> &[u8] {
        self.data.as_slice()
    }

    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.data)
    }

    pub fn is_writable(&self) -> bool {
        self.strong_count() == 1
    }

    pub fn get_mut(&mut self) -> Option<&mut [u8]> {
        Arc::get_mut(&mut self.data).map(Vec::as_mut_slice)
    }

    pub fn make_mut(&mut self) -> &mut [u8] {
        Arc::make_mut(&mut self.data).as_mut_slice()
    }

    pub fn slice(&self, offset: usize, len: usize) -> AvResult<BufferSlice> {
        let end = offset.checked_add(len).ok_or_else(|| {
            AvError::invalid_argument("buffer slice offset plus length overflows")
        })?;
        if offset > self.len() || end > self.len() {
            return Err(AvError::invalid_argument(format!(
                "buffer slice {offset}..{end} exceeds {} bytes",
                self.len()
            )));
        }

        Ok(BufferSlice {
            data: Arc::clone(&self.data),
            offset,
            len,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferSlice {
    data: Arc<Vec<u8>>,
    offset: usize,
    len: usize,
}

impl BufferSlice {
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data[self.offset..self.offset + self.len]
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AvErrorKind;

    #[test]
    fn buffer_ref_wraps_owned_and_copied_bytes() {
        let owned = BufferRef::from_vec(vec![1, 2, 3]);
        assert_eq!(owned.as_slice(), &[1, 2, 3]);
        assert_eq!(owned.len(), 3);
        assert!(!owned.is_empty());
        assert!(owned.is_writable());

        let source = [4, 5, 6];
        let copied = BufferRef::copy_from_slice(&source);
        assert_eq!(copied.as_slice(), &source);
        assert!(copied.is_writable());
    }

    #[test]
    fn zeroed_buffer_allocates_requested_zero_bytes() {
        let empty = BufferRef::zeroed(0).unwrap();
        assert!(empty.is_empty());
        assert_eq!(empty.as_slice(), &[]);

        let buffer = BufferRef::zeroed(4).unwrap();
        assert_eq!(buffer.as_slice(), &[0, 0, 0, 0]);
    }

    #[test]
    fn cloned_buffers_share_until_copy_on_write_mutation() {
        let mut buffer = BufferRef::from_vec(vec![1, 2, 3]);
        let shared = buffer.clone();

        assert_eq!(buffer.strong_count(), 2);
        assert!(!buffer.is_writable());
        assert!(buffer.get_mut().is_none());

        buffer.make_mut()[0] = 9;

        assert_eq!(buffer.as_slice(), &[9, 2, 3]);
        assert_eq!(shared.as_slice(), &[1, 2, 3]);
        assert!(buffer.is_writable());
        assert!(buffer.get_mut().is_some());
    }

    #[test]
    fn buffer_slices_are_checked_views_over_shared_storage() {
        let buffer = BufferRef::from_vec(vec![10, 11, 12, 13]);
        let middle = buffer.slice(1, 2).unwrap();
        let empty_at_end = buffer.slice(4, 0).unwrap();

        assert_eq!(middle.offset(), 1);
        assert_eq!(middle.len(), 2);
        assert_eq!(middle.as_slice(), &[11, 12]);
        assert_eq!(empty_at_end.as_slice(), &[]);
        assert!(empty_at_end.is_empty());
        assert_eq!(buffer.strong_count(), 3);
        assert_eq!(middle.strong_count(), 3);
    }

    #[test]
    fn buffer_slices_reject_out_of_bounds_ranges_without_panics() {
        let buffer = BufferRef::from_vec(vec![1, 2, 3]);

        assert_eq!(
            buffer.slice(4, 0).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            buffer.slice(2, 2).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            buffer.slice(usize::MAX, 1).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
    }
}
