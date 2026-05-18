use crate::{AvError, AvResult};
use std::sync::{Arc, Mutex, Weak};

#[derive(Clone)]
pub struct BufferPoolCallbacks {
    allocate: PoolAllocateCallback,
    release: PoolReleaseCallback,
}

type PoolAllocateCallback = Arc<dyn Fn(usize) -> AvResult<Vec<u8>> + Send + Sync + 'static>;
type PoolReleaseCallback = Arc<dyn Fn(Vec<u8>) + Send + Sync + 'static>;

impl BufferPoolCallbacks {
    pub fn new<A, R>(allocate: A, release: R) -> Self
    where
        A: Fn(usize) -> AvResult<Vec<u8>> + Send + Sync + 'static,
        R: Fn(Vec<u8>) + Send + Sync + 'static,
    {
        Self {
            allocate: Arc::new(allocate),
            release: Arc::new(release),
        }
    }
}

impl std::fmt::Debug for BufferPoolCallbacks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BufferPoolCallbacks")
            .finish_non_exhaustive()
    }
}

impl Default for BufferPoolCallbacks {
    fn default() -> Self {
        Self::new(allocate_zeroed_len, drop)
    }
}

#[derive(Debug, Clone)]
pub struct BufferRef {
    data: Arc<BufferStorage>,
    len: usize,
}

impl PartialEq for BufferRef {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len && self.as_padded_slice() == other.as_padded_slice()
    }
}

impl Eq for BufferRef {}

impl BufferRef {
    pub fn from_vec(data: Vec<u8>) -> Self {
        let len = data.len();
        Self {
            data: Arc::new(BufferStorage::new(data)),
            len,
        }
    }

    pub fn from_vec_readonly(data: Vec<u8>) -> Self {
        let len = data.len();
        Self {
            data: Arc::new(BufferStorage::readonly(data)),
            len,
        }
    }

    pub fn from_vec_with_len_readonly(data: Vec<u8>, len: usize) -> AvResult<Self> {
        if len > data.len() {
            return Err(AvError::invalid_argument(format!(
                "visible buffer length {len} exceeds {} allocated bytes",
                data.len()
            )));
        }
        Ok(Self {
            data: Arc::new(BufferStorage::readonly(data)),
            len,
        })
    }

    pub fn from_vec_with_release_callback<F>(data: Vec<u8>, on_release: F) -> Self
    where
        F: Fn(Vec<u8>) + Send + Sync + 'static,
    {
        let len = data.len();
        Self {
            data: Arc::new(BufferStorage::with_release_callback(data, on_release)),
            len,
        }
    }

    pub fn from_vec_with_release_callback_readonly<F>(data: Vec<u8>, on_release: F) -> Self
    where
        F: Fn(Vec<u8>) + Send + Sync + 'static,
    {
        let len = data.len();
        Self {
            data: Arc::new(BufferStorage::with_release_callback_readonly(
                data, on_release, true,
            )),
            len,
        }
    }

    pub fn from_vec_with_len_and_release_callback<F>(
        data: Vec<u8>,
        len: usize,
        on_release: F,
    ) -> AvResult<Self>
    where
        F: Fn(Vec<u8>) + Send + Sync + 'static,
    {
        if len > data.len() {
            return Err(AvError::invalid_argument(format!(
                "visible buffer length {len} exceeds {} allocated bytes",
                data.len()
            )));
        }
        Ok(Self {
            data: Arc::new(BufferStorage::with_release_callback(data, on_release)),
            len,
        })
    }

    pub fn from_vec_with_len_and_release_callback_readonly<F>(
        data: Vec<u8>,
        len: usize,
        on_release: F,
    ) -> AvResult<Self>
    where
        F: Fn(Vec<u8>) + Send + Sync + 'static,
    {
        if len > data.len() {
            return Err(AvError::invalid_argument(format!(
                "visible buffer length {len} exceeds {} allocated bytes",
                data.len()
            )));
        }
        Ok(Self {
            data: Arc::new(BufferStorage::with_release_callback_readonly(
                data, on_release, true,
            )),
            len,
        })
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
            data: Arc::new(BufferStorage::new(storage)),
            len: data.len(),
        })
    }

    pub fn zeroed(size: usize) -> AvResult<Self> {
        Self::zeroed_with_padding(size, 0)
    }

    pub fn zeroed_with_padding(size: usize, padding: usize) -> AvResult<Self> {
        let data = allocate_zeroed_storage(size, padding)?;
        Ok(Self {
            data: Arc::new(BufferStorage::new(data)),
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
        &self.data.bytes[..self.len]
    }

    pub fn as_padded_slice(&self) -> &[u8] {
        self.data.bytes.as_slice()
    }

    pub fn allocated_len(&self) -> usize {
        self.data.len()
    }

    pub fn padding_len(&self) -> usize {
        self.allocated_len() - self.len
    }

    pub fn padding_slice(&self) -> &[u8] {
        &self.data.bytes[self.len..]
    }

    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.data)
    }

    pub fn is_readonly(&self) -> bool {
        self.data.readonly
    }

    pub fn is_writable(&self) -> bool {
        self.strong_count() == 1 && !self.is_readonly()
    }

    pub fn get_mut(&mut self) -> Option<&mut [u8]> {
        if self.is_readonly() {
            return None;
        }
        Arc::get_mut(&mut self.data).map(|data| &mut data.bytes[..self.len])
    }

    pub fn make_mut(&mut self) -> &mut [u8] {
        if self.strong_count() != 1 || self.is_readonly() {
            let bytes = self.data.bytes.clone();
            self.data = Arc::new(BufferStorage::new(bytes));
        }
        &mut Arc::get_mut(&mut self.data)
            .expect("buffer storage is unique after copy-on-write")
            .bytes[..self.len]
    }

    pub fn resize(&mut self, len: usize) -> AvResult<()> {
        self.resize_with_padding(len, 0)
    }

    pub fn resize_with_padding(&mut self, len: usize, padding: usize) -> AvResult<()> {
        let total_len = checked_storage_len(len, padding)?;
        if len == self.len
            && total_len == self.allocated_len()
            && self.padding_slice().iter().all(|byte| *byte == 0)
        {
            return Ok(());
        }

        if self.can_resize_in_place() {
            let storage =
                Arc::get_mut(&mut self.data).expect("in-place resize requires unique storage");
            if total_len > storage.bytes.len() {
                storage
                    .bytes
                    .try_reserve_exact(total_len - storage.bytes.len())
                    .map_err(|_| {
                        AvError::external(format!(
                            "failed to allocate {total_len} resized buffer bytes"
                        ))
                    })?;
            }
            storage.bytes.resize(total_len, 0);
            storage.bytes[len..].fill(0);
            self.len = len;
            return Ok(());
        }

        let bytes = resized_storage(&self.data.bytes[..self.len], len, padding)?;
        self.data = Arc::new(BufferStorage::new(bytes));
        self.len = len;
        Ok(())
    }

    fn can_resize_in_place(&self) -> bool {
        self.strong_count() == 1 && !self.is_readonly() && self.data.owner.is_none()
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

struct BufferStorage {
    bytes: Vec<u8>,
    owner: Option<BufferOwner>,
    readonly: bool,
}

type BufferReleaseCallback = Arc<dyn Fn(Vec<u8>) + Send + Sync + 'static>;

enum BufferOwner {
    Pool {
        pool: Weak<BufferPoolInner>,
        allocated_len: usize,
        release: PoolReleaseCallback,
    },
    Callback(BufferReleaseCallback),
}

impl BufferStorage {
    fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            owner: None,
            readonly: false,
        }
    }

    fn readonly(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            owner: None,
            readonly: true,
        }
    }

    fn with_pool(bytes: Vec<u8>, pool: &Arc<BufferPoolInner>) -> Self {
        Self {
            bytes,
            owner: Some(BufferOwner::Pool {
                pool: Arc::downgrade(pool),
                allocated_len: pool.allocated_len,
                release: Arc::clone(&pool.callbacks.release),
            }),
            readonly: false,
        }
    }

    fn with_release_callback<F>(bytes: Vec<u8>, on_release: F) -> Self
    where
        F: Fn(Vec<u8>) + Send + Sync + 'static,
    {
        Self::with_release_callback_readonly(bytes, on_release, false)
    }

    fn with_release_callback_readonly<F>(bytes: Vec<u8>, on_release: F, readonly: bool) -> Self
    where
        F: Fn(Vec<u8>) + Send + Sync + 'static,
    {
        Self {
            bytes,
            owner: Some(BufferOwner::Callback(Arc::new(on_release))),
            readonly,
        }
    }

    fn len(&self) -> usize {
        self.bytes.len()
    }

    fn into_vec(mut self) -> Vec<u8> {
        self.owner = None;
        self.readonly = false;
        std::mem::take(&mut self.bytes)
    }
}

impl std::fmt::Debug for BufferStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let owner = match self.owner {
            Some(BufferOwner::Pool { .. }) => "pool",
            Some(BufferOwner::Callback(_)) => "callback",
            None => "none",
        };
        f.debug_struct("BufferStorage")
            .field("bytes", &self.bytes)
            .field("owner", &owner)
            .field("readonly", &self.readonly)
            .finish()
    }
}

impl Clone for BufferStorage {
    fn clone(&self) -> Self {
        Self::new(self.bytes.clone())
    }
}

impl PartialEq for BufferStorage {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl Eq for BufferStorage {}

impl Drop for BufferStorage {
    fn drop(&mut self) {
        match self.owner.take() {
            Some(BufferOwner::Pool {
                pool,
                allocated_len,
                release,
            }) => {
                let mut storage = std::mem::take(&mut self.bytes);
                if storage.len() != allocated_len {
                    release(storage);
                    return;
                }
                storage.fill(0);
                let Some(pool) = pool.upgrade() else {
                    release(storage);
                    return;
                };
                match pool.spare.lock() {
                    Ok(mut spare) => spare.push(storage),
                    Err(_) => release(storage),
                };
            }
            Some(BufferOwner::Callback(on_release)) => {
                let storage = std::mem::take(&mut self.bytes);
                on_release(storage);
            }
            None => {}
        }
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
    callbacks: BufferPoolCallbacks,
    spare: Mutex<Vec<Vec<u8>>>,
}

impl Drop for BufferPoolInner {
    fn drop(&mut self) {
        let spare = match self.spare.get_mut() {
            Ok(spare) => spare,
            Err(poisoned) => poisoned.into_inner(),
        };
        let release = Arc::clone(&self.callbacks.release);
        for mut storage in spare.drain(..) {
            if storage.len() == self.allocated_len {
                storage.fill(0);
            }
            release(storage);
        }
    }
}

impl BufferPool {
    pub fn new(len: usize, padding: usize) -> AvResult<Self> {
        Self::with_callbacks(len, padding, BufferPoolCallbacks::default())
    }

    pub fn with_callbacks(
        len: usize,
        padding: usize,
        callbacks: BufferPoolCallbacks,
    ) -> AvResult<Self> {
        let allocated_len = checked_storage_len(len, padding)?;
        Ok(Self {
            inner: Arc::new(BufferPoolInner {
                len,
                allocated_len,
                padding,
                callbacks,
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
                    data: Arc::new(BufferStorage::with_pool(storage, &self.inner)),
                    len: self.len(),
                })
            }
            None => {
                let storage = self.allocate_storage()?;
                Ok(BufferRef {
                    data: Arc::new(BufferStorage::with_pool(storage, &self.inner)),
                    len: self.len(),
                })
            }
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
        if data.readonly {
            return Err(AvError::invalid_argument(
                "cannot recycle a readonly buffer into a mutable pool",
            ));
        }

        let storage = Arc::try_unwrap(data)
            .map_err(|_| AvError::invalid_argument("cannot recycle a shared buffer"))?;
        let mut storage = storage.into_vec();
        storage.fill(0);
        self.lock_spare()?.push(storage);
        Ok(())
    }

    fn allocate_storage(&self) -> AvResult<Vec<u8>> {
        let mut storage = (self.inner.callbacks.allocate)(self.allocated_len())?;
        if storage.len() != self.allocated_len() {
            let actual_len = storage.len();
            (self.inner.callbacks.release)(storage);
            return Err(AvError::invalid_argument(format!(
                "buffer pool allocator returned {actual_len} bytes for {} byte shape",
                self.allocated_len()
            )));
        }
        storage.fill(0);
        Ok(storage)
    }

    fn lock_spare(&self) -> AvResult<std::sync::MutexGuard<'_, Vec<Vec<u8>>>> {
        self.inner
            .spare
            .lock()
            .map_err(|_| AvError::external("buffer pool lock poisoned"))
    }
}

fn allocate_zeroed_storage(len: usize, padding: usize) -> AvResult<Vec<u8>> {
    let total_len = checked_storage_len(len, padding)?;
    allocate_zeroed_len(total_len)
}

fn allocate_zeroed_len(total_len: usize) -> AvResult<Vec<u8>> {
    let mut data = Vec::new();
    data.try_reserve_exact(total_len).map_err(|_| {
        AvError::external(format!(
            "failed to allocate {total_len} padded buffer bytes"
        ))
    })?;
    data.resize(total_len, 0);
    Ok(data)
}

fn resized_storage(visible: &[u8], len: usize, padding: usize) -> AvResult<Vec<u8>> {
    let total_len = checked_storage_len(len, padding)?;
    let mut storage = Vec::new();
    storage.try_reserve_exact(total_len).map_err(|_| {
        AvError::external(format!(
            "failed to allocate {total_len} resized buffer bytes"
        ))
    })?;
    storage.extend_from_slice(&visible[..visible.len().min(len)]);
    storage.resize(total_len, 0);
    Ok(storage)
}

fn checked_storage_len(len: usize, padding: usize) -> AvResult<usize> {
    len.checked_add(padding)
        .ok_or_else(|| AvError::invalid_argument("buffer length plus padding overflows"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BufferSlice {
    data: Arc<BufferStorage>,
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
        &self.data.bytes[self.offset..self.offset + self.len]
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
    fn callback_owned_buffer_releases_storage_after_last_reference() {
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
        let capture = std::sync::Arc::clone(&released);
        let buffer = BufferRef::from_vec_with_release_callback(vec![1, 2, 3], move |storage| {
            capture.lock().unwrap().push(storage);
        });
        let slice = buffer.slice(1, 2).unwrap();
        let shared = buffer.clone();

        drop(buffer);
        assert!(released.lock().unwrap().is_empty());
        drop(slice);
        assert!(released.lock().unwrap().is_empty());
        drop(shared);

        assert_eq!(*released.lock().unwrap(), vec![vec![1, 2, 3]]);
    }

    #[test]
    fn callback_owned_buffer_supports_visible_len_and_padding() {
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
        let capture = std::sync::Arc::clone(&released);
        let buffer = BufferRef::from_vec_with_len_and_release_callback(
            vec![1, 2, 3, 0, 0],
            3,
            move |storage| {
                capture.lock().unwrap().push(storage);
            },
        )
        .unwrap();

        assert_eq!(buffer.as_slice(), &[1, 2, 3]);
        assert_eq!(buffer.padding_slice(), &[0, 0]);
        drop(buffer);

        assert_eq!(*released.lock().unwrap(), vec![vec![1, 2, 3, 0, 0]]);
    }

    #[test]
    fn callback_owned_buffer_rejects_invalid_visible_len_without_release() {
        let release_count = std::sync::Arc::new(std::sync::Mutex::new(0usize));
        let capture = std::sync::Arc::clone(&release_count);
        assert_eq!(
            BufferRef::from_vec_with_len_and_release_callback(vec![1], 2, move |_| {
                *capture.lock().unwrap() += 1;
            })
            .unwrap_err()
            .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(*release_count.lock().unwrap(), 0);
    }

    #[test]
    fn callback_owned_copy_on_write_releases_only_original_storage() {
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
        let capture = std::sync::Arc::clone(&released);
        let mut buffer = BufferRef::from_vec_with_release_callback(vec![1, 2, 3], move |storage| {
            capture.lock().unwrap().push(storage);
        });
        let shared = buffer.clone();

        buffer.make_mut()[0] = 9;
        assert_eq!(buffer.as_slice(), &[9, 2, 3]);
        assert_eq!(shared.as_slice(), &[1, 2, 3]);
        drop(buffer);
        assert!(released.lock().unwrap().is_empty());
        drop(shared);

        assert_eq!(*released.lock().unwrap(), vec![vec![1, 2, 3]]);
    }

    #[test]
    fn readonly_buffer_copies_before_mutation_and_preserves_padding() {
        let mut buffer = BufferRef::from_vec_with_len_readonly(vec![1, 2, 3, 0, 0], 3).unwrap();
        let shared = buffer.clone();

        assert!(buffer.is_readonly());
        assert!(!buffer.is_writable());
        assert!(buffer.get_mut().is_none());
        assert_eq!(buffer.as_slice(), &[1, 2, 3]);
        assert_eq!(buffer.padding_slice(), &[0, 0]);

        buffer.make_mut()[1] = 9;

        assert_eq!(buffer.as_slice(), &[1, 9, 3]);
        assert_eq!(buffer.padding_slice(), &[0, 0]);
        assert!(!buffer.is_readonly());
        assert!(buffer.is_writable());
        assert!(buffer.get_mut().is_some());
        assert_eq!(shared.as_slice(), &[1, 2, 3]);
        assert_eq!(shared.padding_slice(), &[0, 0]);
        assert!(shared.is_readonly());
        assert!(!shared.is_writable());
    }

    #[test]
    fn readonly_buffer_rejects_invalid_visible_len() {
        assert_eq!(
            BufferRef::from_vec_with_len_readonly(vec![1], 2)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );

        let release_count = std::sync::Arc::new(std::sync::Mutex::new(0usize));
        let capture = std::sync::Arc::clone(&release_count);
        assert_eq!(
            BufferRef::from_vec_with_len_and_release_callback_readonly(vec![1], 2, move |_| {
                *capture.lock().unwrap() += 1;
            })
            .unwrap_err()
            .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(*release_count.lock().unwrap(), 0);
    }

    #[test]
    fn readonly_callback_buffer_make_mut_releases_unique_original_storage() {
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
        let capture = std::sync::Arc::clone(&released);
        let mut buffer = BufferRef::from_vec_with_len_and_release_callback_readonly(
            vec![1, 2, 3, 0],
            3,
            move |storage| {
                capture.lock().unwrap().push(storage);
            },
        )
        .unwrap();

        assert!(buffer.is_readonly());
        assert!(!buffer.is_writable());
        assert!(buffer.get_mut().is_none());

        buffer.make_mut()[0] = 9;

        assert_eq!(*released.lock().unwrap(), vec![vec![1, 2, 3, 0]]);
        assert_eq!(buffer.as_slice(), &[9, 2, 3]);
        assert_eq!(buffer.padding_slice(), &[0]);
        assert!(!buffer.is_readonly());
        assert!(buffer.is_writable());
        drop(buffer);
        assert_eq!(*released.lock().unwrap(), vec![vec![1, 2, 3, 0]]);
    }

    #[test]
    fn readonly_shared_callback_buffer_releases_after_last_original_reference() {
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
        let capture = std::sync::Arc::clone(&released);
        let mut buffer =
            BufferRef::from_vec_with_release_callback_readonly(vec![1, 2, 3], move |storage| {
                capture.lock().unwrap().push(storage);
            });
        let shared = buffer.clone();

        buffer.make_mut()[2] = 7;

        assert!(released.lock().unwrap().is_empty());
        assert_eq!(buffer.as_slice(), &[1, 2, 7]);
        assert!(!buffer.is_readonly());
        assert_eq!(shared.as_slice(), &[1, 2, 3]);
        assert!(shared.is_readonly());
        drop(buffer);
        assert!(released.lock().unwrap().is_empty());
        drop(shared);
        assert_eq!(*released.lock().unwrap(), vec![vec![1, 2, 3]]);
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
    fn buffer_resize_shrinks_grows_and_zeroes_padding() {
        let mut buffer = BufferRef::from_vec(vec![1, 2, 3, 4]);

        buffer.resize_with_padding(2, 2).unwrap();
        assert_eq!(buffer.as_slice(), &[1, 2]);
        assert_eq!(buffer.padding_slice(), &[0, 0]);
        assert_eq!(buffer.allocated_len(), 4);
        assert!(buffer.is_writable());

        buffer.resize_with_padding(5, 3).unwrap();
        assert_eq!(buffer.as_slice(), &[1, 2, 0, 0, 0]);
        assert_eq!(buffer.padding_slice(), &[0, 0, 0]);
        assert_eq!(buffer.allocated_len(), 8);

        buffer.resize(1).unwrap();
        assert_eq!(buffer.as_slice(), &[1]);
        assert_eq!(buffer.padding_slice(), &[]);
        assert_eq!(buffer.allocated_len(), 1);
    }

    #[test]
    fn buffer_resize_detaches_shared_storage_without_mutating_original() {
        let mut buffer = BufferRef::from_vec(vec![1, 2, 3]);
        let shared = buffer.clone();

        buffer.resize_with_padding(5, 1).unwrap();

        assert_eq!(buffer.as_slice(), &[1, 2, 3, 0, 0]);
        assert_eq!(buffer.padding_slice(), &[0]);
        assert!(buffer.is_writable());
        assert_eq!(shared.as_slice(), &[1, 2, 3]);
        assert_eq!(shared.allocated_len(), 3);
    }

    #[test]
    fn buffer_resize_releases_unique_readonly_callback_storage() {
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
        let capture = std::sync::Arc::clone(&released);
        let mut buffer = BufferRef::from_vec_with_len_and_release_callback_readonly(
            vec![1, 2, 3, 0],
            3,
            move |storage| {
                capture.lock().unwrap().push(storage);
            },
        )
        .unwrap();

        buffer.resize_with_padding(2, 1).unwrap();

        assert_eq!(*released.lock().unwrap(), vec![vec![1, 2, 3, 0]]);
        assert_eq!(buffer.as_slice(), &[1, 2]);
        assert_eq!(buffer.padding_slice(), &[0]);
        assert!(!buffer.is_readonly());
        assert!(buffer.is_writable());
    }

    #[test]
    fn buffer_resize_keeps_shared_callback_storage_until_last_original_reference() {
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
        let capture = std::sync::Arc::clone(&released);
        let mut buffer = BufferRef::from_vec_with_release_callback(vec![1, 2, 3], move |storage| {
            capture.lock().unwrap().push(storage);
        });
        let shared = buffer.clone();

        buffer.resize_with_padding(4, 1).unwrap();

        assert!(released.lock().unwrap().is_empty());
        assert_eq!(buffer.as_slice(), &[1, 2, 3, 0]);
        assert_eq!(buffer.padding_slice(), &[0]);
        assert_eq!(shared.as_slice(), &[1, 2, 3]);
        drop(buffer);
        assert!(released.lock().unwrap().is_empty());
        drop(shared);
        assert_eq!(*released.lock().unwrap(), vec![vec![1, 2, 3]]);
    }

    #[test]
    fn buffer_resize_same_shape_zeroes_existing_padding() {
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
        let capture = std::sync::Arc::clone(&released);
        let mut buffer = BufferRef::from_vec_with_len_and_release_callback(
            vec![1, 2, 9, 8],
            2,
            move |storage| {
                capture.lock().unwrap().push(storage);
            },
        )
        .unwrap();

        buffer.resize_with_padding(2, 2).unwrap();

        assert_eq!(*released.lock().unwrap(), vec![vec![1, 2, 9, 8]]);
        assert_eq!(buffer.as_slice(), &[1, 2]);
        assert_eq!(buffer.padding_slice(), &[0, 0]);
        assert!(buffer.is_writable());
    }

    #[test]
    fn buffer_resize_rejects_overflow_without_mutating() {
        let mut buffer = BufferRef::from_vec(vec![1, 2, 3]);

        assert_eq!(
            buffer
                .resize_with_padding(1, usize::MAX)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(buffer.as_slice(), &[1, 2, 3]);
        assert_eq!(buffer.allocated_len(), 3);
        assert!(buffer.is_writable());
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
    fn custom_buffer_pool_callbacks_allocate_reuse_and_release_storage() {
        let allocations = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));
        let releases = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
        let allocate_capture = std::sync::Arc::clone(&allocations);
        let release_capture = std::sync::Arc::clone(&releases);
        let pool = BufferPool::with_callbacks(
            3,
            2,
            BufferPoolCallbacks::new(
                move |allocated_len| {
                    allocate_capture.lock().unwrap().push(allocated_len);
                    Ok(vec![9; allocated_len])
                },
                move |storage| {
                    release_capture.lock().unwrap().push(storage);
                },
            ),
        )
        .unwrap();

        let mut first = pool.get().unwrap();
        assert_eq!(*allocations.lock().unwrap(), vec![5]);
        assert_eq!(first.as_padded_slice(), &[0, 0, 0, 0, 0]);
        first.make_mut().copy_from_slice(&[1, 2, 3]);
        drop(first);
        assert_eq!(pool.available_count().unwrap(), 1);
        assert!(releases.lock().unwrap().is_empty());

        let second = pool.get().unwrap();
        assert_eq!(*allocations.lock().unwrap(), vec![5]);
        assert_eq!(second.as_padded_slice(), &[0, 0, 0, 0, 0]);
        drop(second);
        drop(pool);

        assert_eq!(*releases.lock().unwrap(), vec![vec![0, 0, 0, 0, 0]]);
    }

    #[test]
    fn custom_buffer_pool_releases_outstanding_storage_after_pool_drop() {
        let releases = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
        let release_capture = std::sync::Arc::clone(&releases);
        let pool = BufferPool::with_callbacks(
            2,
            1,
            BufferPoolCallbacks::new(
                |allocated_len| Ok(vec![7; allocated_len]),
                move |storage| {
                    release_capture.lock().unwrap().push(storage);
                },
            ),
        )
        .unwrap();
        let mut buffer = pool.get().unwrap();
        let shared = buffer.clone();

        buffer.make_mut().copy_from_slice(&[4, 5]);
        drop(pool);
        drop(buffer);
        assert!(releases.lock().unwrap().is_empty());
        drop(shared);

        assert_eq!(*releases.lock().unwrap(), vec![vec![0, 0, 0]]);
    }

    #[test]
    fn custom_buffer_pool_rejects_bad_allocator_shape_and_releases_storage() {
        let releases = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
        let release_capture = std::sync::Arc::clone(&releases);
        let pool = BufferPool::with_callbacks(
            2,
            1,
            BufferPoolCallbacks::new(
                |allocated_len| Ok(vec![1; allocated_len + 1]),
                move |storage| {
                    release_capture.lock().unwrap().push(storage);
                },
            ),
        )
        .unwrap();

        assert_eq!(pool.get().unwrap_err().kind(), AvErrorKind::InvalidArgument);
        assert_eq!(*releases.lock().unwrap(), vec![vec![1, 1, 1, 1]]);
        assert_eq!(pool.available_count().unwrap(), 0);
    }

    #[test]
    fn buffer_pool_recycle_transfers_callback_owned_storage_to_pool() {
        let original_releases = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
        let pool_releases = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
        let original_capture = std::sync::Arc::clone(&original_releases);
        let pool_capture = std::sync::Arc::clone(&pool_releases);
        let pool = BufferPool::with_callbacks(
            2,
            1,
            BufferPoolCallbacks::new(
                |allocated_len| Ok(vec![0; allocated_len]),
                move |storage| {
                    pool_capture.lock().unwrap().push(storage);
                },
            ),
        )
        .unwrap();
        let buffer =
            BufferRef::from_vec_with_len_and_release_callback(vec![8, 9, 0], 2, move |storage| {
                original_capture.lock().unwrap().push(storage);
            })
            .unwrap();

        pool.recycle(buffer).unwrap();
        assert!(original_releases.lock().unwrap().is_empty());
        assert_eq!(pool.available_count().unwrap(), 1);
        drop(pool);

        assert!(original_releases.lock().unwrap().is_empty());
        assert_eq!(*pool_releases.lock().unwrap(), vec![vec![0, 0, 0]]);
    }

    #[test]
    fn buffer_pool_recovers_unique_buffers_when_last_reference_drops() {
        let pool = BufferPool::new(4, 2).unwrap();

        {
            let mut buffer = pool.get().unwrap();
            buffer.make_mut().copy_from_slice(&[1, 2, 3, 4]);
            assert_eq!(buffer.padding_slice(), &[0, 0]);
            assert_eq!(pool.available_count().unwrap(), 0);
        }

        assert_eq!(pool.available_count().unwrap(), 1);
        let reused = pool.get().unwrap();
        assert_eq!(pool.available_count().unwrap(), 0);
        assert_eq!(reused.as_slice(), &[0, 0, 0, 0]);
        assert_eq!(reused.padding_slice(), &[0, 0]);
    }

    #[test]
    fn buffer_pool_waits_until_last_shared_reference_drops() {
        let pool = BufferPool::new(2, 1).unwrap();
        let buffer = pool.get().unwrap();
        let shared = buffer.clone();

        drop(buffer);
        assert_eq!(pool.available_count().unwrap(), 0);

        drop(shared);
        assert_eq!(pool.available_count().unwrap(), 1);
    }

    #[test]
    fn buffer_pool_copy_on_write_returns_only_original_storage() {
        let pool = BufferPool::new(3, 1).unwrap();
        let mut buffer = pool.get().unwrap();
        let shared = buffer.clone();

        buffer.make_mut().copy_from_slice(&[7, 8, 9]);
        assert_eq!(buffer.as_slice(), &[7, 8, 9]);
        assert_eq!(shared.as_slice(), &[0, 0, 0]);

        drop(buffer);
        assert_eq!(pool.available_count().unwrap(), 0);

        drop(shared);
        assert_eq!(pool.available_count().unwrap(), 1);
    }

    #[test]
    fn buffer_resize_detaches_pool_owned_storage_back_to_pool() {
        let pool = BufferPool::new(2, 1).unwrap();
        let mut buffer = pool.get().unwrap();

        buffer.make_mut().copy_from_slice(&[8, 9]);
        buffer.resize_with_padding(3, 0).unwrap();

        assert_eq!(buffer.as_slice(), &[8, 9, 0]);
        assert_eq!(buffer.padding_slice(), &[]);
        assert_eq!(pool.available_count().unwrap(), 1);
        drop(buffer);
        assert_eq!(pool.available_count().unwrap(), 1);

        let reused = pool.get().unwrap();
        assert_eq!(reused.as_padded_slice(), &[0, 0, 0]);
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
    fn buffer_pool_rejects_readonly_buffers() {
        let pool = BufferPool::new(2, 1).unwrap();
        let buffer = BufferRef::from_vec_with_len_readonly(vec![1, 2, 0], 2).unwrap();

        assert_eq!(
            pool.recycle(buffer).unwrap_err().kind(),
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
