use crate::{AvError, AvResult};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferRef {
    data: Arc<Vec<u8>>,
    len: usize,
}

impl BufferRef {
    pub fn from_vec(data: Vec<u8>) -> Self {
        let len = data.len();
        Self {
            data: Arc::new(data),
            len,
        }
    }

    pub fn copy_from_slice(data: &[u8]) -> Self {
        Self::from_vec(data.to_vec())
    }

    pub fn copy_from_slice_with_padding(data: &[u8], padding: usize) -> AvResult<Self> {
        let total_len = checked_storage_len(data.len(), padding)?;
        let mut storage = Vec::new();
        storage.try_reserve_exact(total_len).map_err(|_| {
            AvError::external(format!(
                "failed to allocate {total_len} padded buffer bytes"
            ))
        })?;
        storage.extend_from_slice(data);
        storage.resize(total_len, 0);
        Ok(Self {
            data: Arc::new(storage),
            len: data.len(),
        })
    }

    pub fn zeroed(size: usize) -> AvResult<Self> {
        Self::zeroed_with_padding(size, 0)
    }

    pub fn zeroed_with_padding(size: usize, padding: usize) -> AvResult<Self> {
        let total_len = checked_storage_len(size, padding)?;
        let mut data = Vec::new();
        data.try_reserve_exact(total_len).map_err(|_| {
            AvError::external(format!(
                "failed to allocate {total_len} padded buffer bytes"
            ))
        })?;
        data.resize(total_len, 0);
        Ok(Self {
            data: Arc::new(data),
            len: size,
        })
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.len]
    }

    pub fn as_padded_slice(&self) -> &[u8] {
        self.data.as_slice()
    }

    pub fn allocated_len(&self) -> usize {
        self.data.len()
    }

    pub fn padding_len(&self) -> usize {
        self.allocated_len() - self.len
    }

    pub fn padding_slice(&self) -> &[u8] {
        &self.data[self.len..]
    }

    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.data)
    }

    pub fn is_writable(&self) -> bool {
        self.strong_count() == 1
    }

    pub fn get_mut(&mut self) -> Option<&mut [u8]> {
        Arc::get_mut(&mut self.data).map(|data| &mut data[..self.len])
    }

    pub fn make_mut(&mut self) -> &mut [u8] {
        &mut Arc::make_mut(&mut self.data)[..self.len]
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

#[derive(Debug, Clone)]
pub struct BufferPool {
    inner: Arc<BufferPoolInner>,
}

#[derive(Debug)]
struct BufferPoolInner {
    len: usize,
    allocated_len: usize,
    padding: usize,
    spare: Mutex<Vec<Vec<u8>>>,
}

impl BufferPool {
    pub fn new(len: usize, padding: usize) -> AvResult<Self> {
        let allocated_len = checked_storage_len(len, padding)?;
        Ok(Self {
            inner: Arc::new(BufferPoolInner {
                len,
                allocated_len,
                padding,
                spare: Mutex::new(Vec::new()),
            }),
        })
    }

    pub fn len(&self) -> usize {
        self.inner.len
    }

    pub fn is_empty(&self) -> bool {
        self.inner.len == 0
    }

    pub fn allocated_len(&self) -> usize {
        self.inner.allocated_len
    }

    pub fn padding_len(&self) -> usize {
        self.inner.padding
    }

    pub fn available_count(&self) -> AvResult<usize> {
        Ok(self.lock_spare()?.len())
    }

    pub fn get(&self) -> AvResult<BufferRef> {
        let storage = self.lock_spare()?.pop();
        match storage {
            Some(mut storage) => {
                storage.resize(self.allocated_len(), 0);
                storage.fill(0);
                Ok(BufferRef {
                    data: Arc::new(storage),
                    len: self.len(),
                })
            }
            None => BufferRef::zeroed_with_padding(self.len(), self.padding_len()),
        }
    }

    pub fn recycle(&self, buffer: BufferRef) -> AvResult<()> {
        let BufferRef { data, len } = buffer;
        if len != self.len() || data.len() != self.allocated_len() {
            return Err(AvError::invalid_argument(format!(
                "buffer shape {len}/{} does not match pool shape {}/{}",
                data.len(),
                self.len(),
                self.allocated_len()
            )));
        }

        let mut storage = Arc::try_unwrap(data)
            .map_err(|_| AvError::invalid_argument("cannot recycle a shared buffer"))?;
        storage.fill(0);
        self.lock_spare()?.push(storage);
        Ok(())
    }

    fn lock_spare(&self) -> AvResult<std::sync::MutexGuard<'_, Vec<Vec<u8>>>> {
        self.inner
            .spare
            .lock()
            .map_err(|_| AvError::external("buffer pool lock poisoned"))
    }
}

fn checked_storage_len(len: usize, padding: usize) -> AvResult<usize> {
    len.checked_add(padding)
        .ok_or_else(|| AvError::invalid_argument("buffer length plus padding overflows"))
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
        assert_eq!(empty.as_padded_slice(), &[]);

        let buffer = BufferRef::zeroed(4).unwrap();
        assert_eq!(buffer.as_slice(), &[0, 0, 0, 0]);
        assert_eq!(buffer.allocated_len(), 4);
        assert_eq!(buffer.padding_len(), 0);
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

    #[test]
    fn padded_buffers_expose_visible_bytes_and_zeroed_tail() {
        let buffer = BufferRef::copy_from_slice_with_padding(&[1, 2, 3], 4).unwrap();
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.allocated_len(), 7);
        assert_eq!(buffer.padding_len(), 4);
        assert_eq!(buffer.as_slice(), &[1, 2, 3]);
        assert_eq!(buffer.padding_slice(), &[0, 0, 0, 0]);
        assert_eq!(buffer.as_padded_slice(), &[1, 2, 3, 0, 0, 0, 0]);

        let zeroed = BufferRef::zeroed_with_padding(2, 3).unwrap();
        assert_eq!(zeroed.len(), 2);
        assert_eq!(zeroed.as_slice(), &[0, 0]);
        assert_eq!(zeroed.padding_slice(), &[0, 0, 0]);
        assert_eq!(zeroed.as_padded_slice(), &[0, 0, 0, 0, 0]);
    }

    #[test]
    fn visible_slices_do_not_expose_padding_bytes() {
        let buffer = BufferRef::copy_from_slice_with_padding(&[1, 2, 3], 8).unwrap();

        assert_eq!(buffer.slice(3, 0).unwrap().as_slice(), &[]);
        assert_eq!(
            buffer.slice(4, 0).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            buffer.slice(2, 2).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
    }

    #[test]
    fn padded_copy_on_write_keeps_padding_zeroed_and_isolated() {
        let mut buffer = BufferRef::copy_from_slice_with_padding(&[1, 2, 3], 2).unwrap();
        let shared = buffer.clone();

        buffer.make_mut()[1] = 9;

        assert_eq!(buffer.as_slice(), &[1, 9, 3]);
        assert_eq!(buffer.padding_slice(), &[0, 0]);
        assert_eq!(shared.as_slice(), &[1, 2, 3]);
        assert_eq!(shared.padding_slice(), &[0, 0]);
    }

    #[test]
    fn padded_constructors_reject_length_overflow() {
        assert_eq!(
            BufferRef::copy_from_slice_with_padding(&[1], usize::MAX)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            BufferRef::zeroed_with_padding(1, usize::MAX)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
    }

    #[test]
    fn buffer_pool_allocates_recycles_and_reuses_zeroed_storage() {
        let pool = BufferPool::new(3, 2).unwrap();

        assert_eq!(pool.len(), 3);
        assert_eq!(pool.allocated_len(), 5);
        assert_eq!(pool.padding_len(), 2);
        assert!(!pool.is_empty());
        assert_eq!(pool.available_count().unwrap(), 0);

        let mut buffer = pool.get().unwrap();
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.allocated_len(), 5);
        assert_eq!(buffer.as_padded_slice(), &[0, 0, 0, 0, 0]);
        buffer.make_mut().copy_from_slice(&[4, 5, 6]);
        assert_eq!(buffer.padding_slice(), &[0, 0]);

        pool.recycle(buffer).unwrap();
        assert_eq!(pool.available_count().unwrap(), 1);

        let reused = pool.get().unwrap();
        assert_eq!(pool.available_count().unwrap(), 0);
        assert_eq!(reused.as_slice(), &[0, 0, 0]);
        assert_eq!(reused.padding_slice(), &[0, 0]);
    }

    #[test]
    fn buffer_pool_rejects_shared_and_wrong_shape_buffers() {
        let pool = BufferPool::new(2, 1).unwrap();
        let buffer = pool.get().unwrap();
        let shared = buffer.clone();

        assert_eq!(
            pool.recycle(buffer).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(shared.as_padded_slice(), &[0, 0, 0]);
        assert_eq!(pool.available_count().unwrap(), 0);

        let wrong_len = BufferRef::zeroed_with_padding(3, 1).unwrap();
        assert_eq!(
            pool.recycle(wrong_len).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        let wrong_padding = BufferRef::zeroed_with_padding(2, 2).unwrap();
        assert_eq!(
            pool.recycle(wrong_padding).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(pool.available_count().unwrap(), 0);
    }

    #[test]
    fn buffer_pool_rejects_overflowing_shapes() {
        assert_eq!(
            BufferPool::new(1, usize::MAX).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
    }
}
