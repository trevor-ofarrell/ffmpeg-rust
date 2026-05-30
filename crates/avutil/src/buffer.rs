use crate::{AvError, AvErrorCode, AvErrorKind, AvResult};
use std::any::Any;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferAbiField {
    pub name: &'static str,
    pub offset: usize,
    pub size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferAbiLayout {
    pub name: &'static str,
    pub size: usize,
    pub align: usize,
    pub fields: &'static [BufferAbiField],
}

pub const AV_BUFFER_REF_ABI_LAYOUT: BufferAbiLayout = BufferAbiLayout {
    name: "AVBufferRef",
    size: 24,
    align: 8,
    fields: &[
        BufferAbiField {
            name: "buffer",
            offset: 0,
            size: 8,
        },
        BufferAbiField {
            name: "data",
            offset: 8,
            size: 8,
        },
        BufferAbiField {
            name: "size",
            offset: 16,
            size: 8,
        },
    ],
};

pub const AV_BUFFER_FLAG_READONLY: i32 = 1 << 0;

#[derive(Clone)]
pub struct BufferPoolCallbacks {
    allocate: PoolAllocateCallback,
    release: PoolReleaseCallback,
    pool_free: Option<PoolFreeCallback>,
}

type PoolAllocateCallback =
    Arc<dyn Fn(usize) -> AvResult<BufferPoolAllocation> + Send + Sync + 'static>;
type PoolReleaseCallback = Arc<dyn Fn(BufferPoolAllocation) + Send + Sync + 'static>;
type PoolFreeCallback = Arc<dyn Fn() + Send + Sync + 'static>;

pub struct BufferPoolAllocation {
    bytes: Vec<u8>,
    opaque: Option<Box<dyn Any + Send + Sync>>,
    readonly: bool,
    visible_offset: usize,
    visible_len: Option<usize>,
    allow_oversized_recycle: bool,
}

impl BufferPoolAllocation {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            opaque: None,
            readonly: false,
            visible_offset: 0,
            visible_len: None,
            allow_oversized_recycle: false,
        }
    }

    pub fn with_opaque<T>(bytes: Vec<u8>, opaque: T) -> Self
    where
        T: Any + Send + Sync + 'static,
    {
        Self {
            bytes,
            opaque: Some(Box::new(opaque)),
            readonly: false,
            visible_offset: 0,
            visible_len: None,
            allow_oversized_recycle: false,
        }
    }

    pub fn readonly(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            opaque: None,
            readonly: true,
            visible_offset: 0,
            visible_len: None,
            allow_oversized_recycle: false,
        }
    }

    pub fn with_opaque_readonly<T>(bytes: Vec<u8>, opaque: T) -> Self
    where
        T: Any + Send + Sync + 'static,
    {
        Self {
            bytes,
            opaque: Some(Box::new(opaque)),
            readonly: true,
            visible_offset: 0,
            visible_len: None,
            allow_oversized_recycle: false,
        }
    }

    pub fn with_visible_range(bytes: Vec<u8>, offset: usize, len: usize) -> AvResult<Self> {
        Self::validate_visible_range(bytes.len(), offset, len)?;
        Ok(Self {
            bytes,
            opaque: None,
            readonly: false,
            visible_offset: offset,
            visible_len: Some(len),
            allow_oversized_recycle: true,
        })
    }

    pub fn readonly_visible_range(bytes: Vec<u8>, offset: usize, len: usize) -> AvResult<Self> {
        Self::validate_visible_range(bytes.len(), offset, len)?;
        Ok(Self {
            bytes,
            opaque: None,
            readonly: true,
            visible_offset: offset,
            visible_len: Some(len),
            allow_oversized_recycle: true,
        })
    }

    pub fn with_opaque_visible_range<T>(
        bytes: Vec<u8>,
        offset: usize,
        len: usize,
        opaque: T,
    ) -> AvResult<Self>
    where
        T: Any + Send + Sync + 'static,
    {
        Self::validate_visible_range(bytes.len(), offset, len)?;
        Ok(Self {
            bytes,
            opaque: Some(Box::new(opaque)),
            readonly: false,
            visible_offset: offset,
            visible_len: Some(len),
            allow_oversized_recycle: true,
        })
    }

    pub fn with_opaque_readonly_visible_range<T>(
        bytes: Vec<u8>,
        offset: usize,
        len: usize,
        opaque: T,
    ) -> AvResult<Self>
    where
        T: Any + Send + Sync + 'static,
    {
        Self::validate_visible_range(bytes.len(), offset, len)?;
        Ok(Self {
            bytes,
            opaque: Some(Box::new(opaque)),
            readonly: true,
            visible_offset: offset,
            visible_len: Some(len),
            allow_oversized_recycle: true,
        })
    }

    fn validate_visible_range(storage_len: usize, offset: usize, len: usize) -> AvResult<()> {
        let end = offset.checked_add(len).ok_or_else(|| {
            AvError::invalid_argument("buffer pool allocation visible range overflows")
        })?;
        if end > storage_len {
            return Err(AvError::invalid_argument(format!(
                "buffer pool allocation visible range {offset}..{end} exceeds {storage_len} bytes"
            )));
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.bytes
    }

    pub fn is_readonly(&self) -> bool {
        self.readonly
    }

    pub fn visible_offset(&self) -> usize {
        self.visible_offset
    }

    pub fn visible_len(&self) -> Option<usize> {
        self.visible_len
    }

    pub fn opaque_ref<T: 'static>(&self) -> Option<&T> {
        self.opaque.as_deref()?.downcast_ref::<T>()
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.bytes
    }
}

impl std::fmt::Debug for BufferPoolAllocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BufferPoolAllocation")
            .field("bytes", &self.bytes)
            .field("has_opaque", &self.opaque.is_some())
            .field("readonly", &self.readonly)
            .field("visible_offset", &self.visible_offset)
            .field("visible_len", &self.visible_len)
            .field("allow_oversized_recycle", &self.allow_oversized_recycle)
            .finish()
    }
}

impl BufferPoolCallbacks {
    pub fn new<A, R>(allocate: A, release: R) -> Self
    where
        A: Fn(usize) -> AvResult<Vec<u8>> + Send + Sync + 'static,
        R: Fn(Vec<u8>) + Send + Sync + 'static,
    {
        Self::with_allocation_callbacks(
            move |allocated_len| allocate(allocated_len).map(BufferPoolAllocation::new),
            move |allocation| release(allocation.into_vec()),
        )
    }

    pub fn with_allocation_callbacks<A, R>(allocate: A, release: R) -> Self
    where
        A: Fn(usize) -> AvResult<BufferPoolAllocation> + Send + Sync + 'static,
        R: Fn(BufferPoolAllocation) + Send + Sync + 'static,
    {
        Self {
            allocate: Arc::new(allocate),
            release: Arc::new(release),
            pool_free: None,
        }
    }

    pub fn with_pool_free<F>(mut self, pool_free: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.pool_free = Some(Arc::new(pool_free));
        self
    }

    fn release(&self, allocation: BufferPoolAllocation) {
        (self.release)(allocation);
    }

    fn free_pool(&self) {
        if let Some(pool_free) = &self.pool_free {
            pool_free();
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
    offset: usize,
    len: usize,
}

impl PartialEq for BufferRef {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len && self.as_slice() == other.as_slice()
    }
}

impl Eq for BufferRef {}

impl BufferRef {
    pub fn from_vec(data: Vec<u8>) -> Self {
        let len = data.len();
        Self {
            data: Arc::new(BufferStorage::new(data)),
            offset: 0,
            len,
        }
    }

    pub fn from_vec_readonly(data: Vec<u8>) -> Self {
        let len = data.len();
        Self {
            data: Arc::new(BufferStorage::readonly(data)),
            offset: 0,
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
            offset: 0,
            len,
        })
    }

    pub fn from_static_slice_readonly(data: &'static [u8]) -> Self {
        let len = data.len();
        Self {
            data: Arc::new(BufferStorage::static_readonly(data)),
            offset: 0,
            len,
        }
    }

    pub fn from_static_slice_with_len_readonly(data: &'static [u8], len: usize) -> AvResult<Self> {
        if len > data.len() {
            return Err(AvError::invalid_argument(format!(
                "visible buffer length {len} exceeds {} allocated bytes",
                data.len()
            )));
        }
        Ok(Self {
            data: Arc::new(BufferStorage::static_readonly(data)),
            offset: 0,
            len,
        })
    }

    pub fn from_shared_slice_readonly(data: Arc<[u8]>) -> Self {
        let len = data.len();
        Self {
            data: Arc::new(BufferStorage::shared_readonly(data)),
            offset: 0,
            len,
        }
    }

    pub fn from_shared_slice_with_len_readonly(data: Arc<[u8]>, len: usize) -> AvResult<Self> {
        if len > data.len() {
            return Err(AvError::invalid_argument(format!(
                "visible buffer length {len} exceeds {} allocated bytes",
                data.len()
            )));
        }
        Ok(Self {
            data: Arc::new(BufferStorage::shared_readonly(data)),
            offset: 0,
            len,
        })
    }

    pub fn from_external_slice_with_opaque_readonly<T, F>(
        data: Arc<[u8]>,
        opaque: T,
        on_release: F,
    ) -> Self
    where
        T: Any + Send + Sync + 'static,
        F: FnOnce(T) + Send + Sync + 'static,
    {
        let len = data.len();
        Self {
            data: Arc::new(BufferStorage::with_opaque_release_readonly(
                data, opaque, on_release,
            )),
            offset: 0,
            len,
        }
    }

    pub fn from_external_slice_with_len_and_opaque_readonly<T, F>(
        data: Arc<[u8]>,
        len: usize,
        opaque: T,
        on_release: F,
    ) -> AvResult<Self>
    where
        T: Any + Send + Sync + 'static,
        F: FnOnce(T) + Send + Sync + 'static,
    {
        if len > data.len() {
            return Err(AvError::invalid_argument(format!(
                "visible buffer length {len} exceeds {} allocated bytes",
                data.len()
            )));
        }
        Ok(Self {
            data: Arc::new(BufferStorage::with_opaque_release_readonly(
                data, opaque, on_release,
            )),
            offset: 0,
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
            offset: 0,
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
            offset: 0,
            len,
        }
    }

    pub fn from_vec_with_opaque<T>(data: Vec<u8>, opaque: T) -> Self
    where
        T: Any + Send + Sync + 'static,
    {
        let len = data.len();
        Self {
            data: Arc::new(BufferStorage::with_opaque(data, opaque, false)),
            offset: 0,
            len,
        }
    }

    pub fn from_vec_with_opaque_readonly<T>(data: Vec<u8>, opaque: T) -> Self
    where
        T: Any + Send + Sync + 'static,
    {
        let len = data.len();
        Self {
            data: Arc::new(BufferStorage::with_opaque(data, opaque, true)),
            offset: 0,
            len,
        }
    }

    pub fn from_vec_with_opaque_release_callback<T, F>(
        data: Vec<u8>,
        opaque: T,
        on_release: F,
    ) -> Self
    where
        T: Any + Send + Sync + 'static,
        F: FnOnce(T, Vec<u8>) + Send + Sync + 'static,
    {
        let len = data.len();
        Self {
            data: Arc::new(BufferStorage::with_opaque_data_release(
                data, opaque, on_release, false,
            )),
            offset: 0,
            len,
        }
    }

    pub fn from_vec_with_opaque_release_callback_readonly<T, F>(
        data: Vec<u8>,
        opaque: T,
        on_release: F,
    ) -> Self
    where
        T: Any + Send + Sync + 'static,
        F: FnOnce(T, Vec<u8>) + Send + Sync + 'static,
    {
        let len = data.len();
        Self {
            data: Arc::new(BufferStorage::with_opaque_data_release(
                data, opaque, on_release, true,
            )),
            offset: 0,
            len,
        }
    }

    pub fn from_vec_with_len_and_opaque_release_callback<T, F>(
        data: Vec<u8>,
        len: usize,
        opaque: T,
        on_release: F,
    ) -> AvResult<Self>
    where
        T: Any + Send + Sync + 'static,
        F: FnOnce(T, Vec<u8>) + Send + Sync + 'static,
    {
        if len > data.len() {
            return Err(AvError::invalid_argument(format!(
                "visible buffer length {len} exceeds {} allocated bytes",
                data.len()
            )));
        }
        Ok(Self {
            data: Arc::new(BufferStorage::with_opaque_data_release(
                data, opaque, on_release, false,
            )),
            offset: 0,
            len,
        })
    }

    pub fn from_vec_with_len_and_opaque_release_callback_readonly<T, F>(
        data: Vec<u8>,
        len: usize,
        opaque: T,
        on_release: F,
    ) -> AvResult<Self>
    where
        T: Any + Send + Sync + 'static,
        F: FnOnce(T, Vec<u8>) + Send + Sync + 'static,
    {
        if len > data.len() {
            return Err(AvError::invalid_argument(format!(
                "visible buffer length {len} exceeds {} allocated bytes",
                data.len()
            )));
        }
        Ok(Self {
            data: Arc::new(BufferStorage::with_opaque_data_release(
                data, opaque, on_release, true,
            )),
            offset: 0,
            len,
        })
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
            offset: 0,
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
            offset: 0,
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
            allocation_error(format!(
                "failed to allocate {total_len} padded buffer bytes"
            ))
        })?;
        storage.extend_from_slice(data);
        storage.resize(total_len, 0);
        Ok(Self {
            data: Arc::new(BufferStorage::new(storage)),
            offset: 0,
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
            offset: 0,
            len: size,
        })
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data.bytes.as_slice()[self.offset..self.offset + self.len]
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.as_slice().as_ptr()
    }

    pub fn as_padded_slice(&self) -> &[u8] {
        &self.data.bytes.as_slice()[self.offset..]
    }

    pub fn as_padded_ptr(&self) -> *const u8 {
        self.as_padded_slice().as_ptr()
    }

    pub fn allocated_len(&self) -> usize {
        self.data.len() - self.offset
    }

    pub fn padding_len(&self) -> usize {
        self.allocated_len() - self.len
    }

    pub fn padding_slice(&self) -> &[u8] {
        &self.as_padded_slice()[self.len..]
    }

    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.data)
    }

    pub fn shares_storage(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.data, &other.data)
    }

    pub fn shares_storage_with_slice(&self, slice: &BufferSlice) -> bool {
        Arc::ptr_eq(&self.data, &slice.data)
    }

    pub fn is_readonly(&self) -> bool {
        self.data.readonly
    }

    pub fn is_writable(&self) -> bool {
        self.strong_count() == 1 && !self.is_readonly()
    }

    pub fn opaque_ref<T: 'static>(&self) -> Option<&T> {
        self.data.opaque_ref::<T>()
    }

    pub fn pool_opaque_ref<T: 'static>(&self) -> Option<&T> {
        self.data.pool_opaque_ref::<T>()
    }

    pub fn get_mut(&mut self) -> Option<&mut [u8]> {
        if self.is_readonly() {
            return None;
        }
        let offset = self.offset;
        let end = self.offset + self.len;
        Arc::get_mut(&mut self.data)
            .and_then(|data| data.bytes.as_mut_vec())
            .map(|bytes| &mut bytes[offset..end])
    }

    pub fn make_mut(&mut self) -> &mut [u8] {
        if self.strong_count() != 1 || self.is_readonly() {
            let bytes = self.as_padded_slice().to_vec();
            self.data = Arc::new(BufferStorage::new(bytes));
            self.offset = 0;
        }
        let bytes = Arc::get_mut(&mut self.data)
            .expect("buffer storage is unique after copy-on-write")
            .bytes
            .as_mut_vec()
            .expect("copy-on-write storage is owned");
        &mut bytes[self.offset..self.offset + self.len]
    }

    pub fn resize(&mut self, len: usize) -> AvResult<()> {
        self.resize_with_padding(len, 0)
    }

    pub fn resize_with_padding(&mut self, len: usize, padding: usize) -> AvResult<()> {
        let total_len = checked_storage_len(len, padding)?;
        let can_resize_in_place = self.can_resize_in_place();
        if len == self.len
            && total_len == self.allocated_len()
            && self.padding_slice().iter().all(|byte| *byte == 0)
            && can_resize_in_place
        {
            return Ok(());
        }

        if can_resize_in_place {
            let storage =
                Arc::get_mut(&mut self.data).expect("in-place resize requires unique storage");
            let bytes = storage
                .bytes
                .as_mut_vec()
                .expect("in-place resize requires owned storage");
            if total_len > bytes.len() {
                bytes
                    .try_reserve_exact(total_len - bytes.len())
                    .map_err(|_| {
                        allocation_error(format!(
                            "failed to allocate {total_len} resized buffer bytes"
                        ))
                    })?;
            }
            bytes.resize(total_len, 0);
            bytes[len..].fill(0);
            self.len = len;
            return Ok(());
        }

        let bytes = resized_storage(self.as_slice(), len, padding)?;
        self.data = Arc::new(BufferStorage::new(bytes));
        self.offset = 0;
        self.len = len;
        Ok(())
    }

    fn can_resize_in_place(&self) -> bool {
        self.strong_count() == 1
            && self.offset == 0
            && !self.is_readonly()
            && self.data.owner.is_none()
            && self.data.bytes.is_owned()
    }

    pub fn ref_from(source: &Self) -> Self {
        source.clone()
    }

    pub fn replace(dst: &mut Option<Self>, src: Option<&Self>) {
        *dst = src.cloned();
    }

    pub fn realloc(dst: &mut Option<Self>, len: usize) -> AvResult<()> {
        match dst {
            Some(buffer) if buffer.len == len => Ok(()),
            Some(buffer) => buffer.realloc_visible_len(len),
            None => {
                let data = allocate_zeroed_len(len)?;
                *dst = Some(Self {
                    data: Arc::new(BufferStorage::reallocatable(data)),
                    offset: 0,
                    len,
                });
                Ok(())
            }
        }
    }

    fn realloc_visible_len(&mut self, len: usize) -> AvResult<()> {
        if self.can_realloc_in_place() {
            let storage =
                Arc::get_mut(&mut self.data).expect("in-place realloc requires unique storage");
            let bytes = storage
                .bytes
                .as_mut_vec()
                .expect("in-place realloc requires owned storage");
            if len > bytes.len() {
                bytes.try_reserve_exact(len - bytes.len()).map_err(|_| {
                    allocation_error(format!("failed to allocate {len} reallocated buffer bytes"))
                })?;
            }
            bytes.resize(len, 0);
            self.len = len;
            return Ok(());
        }

        let bytes = resized_storage(self.as_slice(), len, 0)?;
        self.data = Arc::new(BufferStorage::reallocatable(bytes));
        self.offset = 0;
        self.len = len;
        Ok(())
    }

    fn can_realloc_in_place(&self) -> bool {
        self.strong_count() == 1
            && self.offset == 0
            && !self.is_readonly()
            && self.data.owner.is_none()
            && self.data.reallocatable
            && self.data.bytes.is_owned()
    }

    pub fn unref(dst: &mut Option<Self>) {
        *dst = None;
    }

    pub fn ref_slice(&self, offset: usize, len: usize) -> AvResult<Self> {
        let end = offset.checked_add(len).ok_or_else(|| {
            AvError::invalid_argument("buffer ref slice offset plus length overflows")
        })?;
        if offset > self.len() || end > self.len() {
            return Err(AvError::invalid_argument(format!(
                "buffer ref slice {offset}..{end} exceeds {} bytes",
                self.len()
            )));
        }

        Ok(Self {
            data: Arc::clone(&self.data),
            offset: self.offset + offset,
            len,
        })
    }

    pub fn into_ref_slice(self, offset: usize, len: usize) -> AvResult<Self> {
        let end = offset.checked_add(len).ok_or_else(|| {
            AvError::invalid_argument("buffer ref slice offset plus length overflows")
        })?;
        if offset > self.len || end > self.len {
            return Err(AvError::invalid_argument(format!(
                "buffer ref slice {offset}..{end} exceeds {} bytes",
                self.len
            )));
        }

        Ok(Self {
            data: self.data,
            offset: self.offset + offset,
            len,
        })
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
            offset: self.offset + offset,
            len,
        })
    }
}

struct BufferStorage {
    bytes: BufferBytes,
    owner: Option<BufferOwner>,
    readonly: bool,
    reallocatable: bool,
}

type BufferReleaseCallback = Arc<dyn Fn(Vec<u8>) + Send + Sync + 'static>;

enum BufferOwner {
    Pool {
        pool: Arc<BufferPoolInner>,
        allocated_len: usize,
        opaque: Option<Box<dyn Any + Send + Sync>>,
        allow_oversized_recycle: bool,
    },
    Callback(BufferReleaseCallback),
    Opaque(Box<dyn OpaqueOwner>),
    OpaqueData(Box<dyn OpaqueDataOwner>),
}

trait OpaqueOwner: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn release(self: Box<Self>);
}

struct TypedOpaqueOwner<T, F> {
    opaque: T,
    on_release: F,
}

trait OpaqueDataOwner: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn release(self: Box<Self>, bytes: Vec<u8>);
}

struct TypedOpaqueDataOwner<T, F> {
    opaque: T,
    on_release: F,
}

impl<T, F> OpaqueOwner for TypedOpaqueOwner<T, F>
where
    T: Any + Send + Sync + 'static,
    F: FnOnce(T) + Send + Sync + 'static,
{
    fn as_any(&self) -> &dyn Any {
        &self.opaque
    }

    fn release(self: Box<Self>) {
        let Self { opaque, on_release } = *self;
        on_release(opaque);
    }
}

impl<T, F> OpaqueDataOwner for TypedOpaqueDataOwner<T, F>
where
    T: Any + Send + Sync + 'static,
    F: FnOnce(T, Vec<u8>) + Send + Sync + 'static,
{
    fn as_any(&self) -> &dyn Any {
        &self.opaque
    }

    fn release(self: Box<Self>, bytes: Vec<u8>) {
        let Self { opaque, on_release } = *self;
        on_release(opaque, bytes);
    }
}

impl BufferStorage {
    fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes: BufferBytes::owned(bytes),
            owner: None,
            readonly: false,
            reallocatable: false,
        }
    }

    fn reallocatable(bytes: Vec<u8>) -> Self {
        Self {
            bytes: BufferBytes::owned(bytes),
            owner: None,
            readonly: false,
            reallocatable: true,
        }
    }

    fn readonly(bytes: Vec<u8>) -> Self {
        Self {
            bytes: BufferBytes::owned(bytes),
            owner: None,
            readonly: true,
            reallocatable: false,
        }
    }

    fn static_readonly(bytes: &'static [u8]) -> Self {
        Self {
            bytes: BufferBytes::static_slice(bytes),
            owner: None,
            readonly: true,
            reallocatable: false,
        }
    }

    fn shared_readonly(bytes: Arc<[u8]>) -> Self {
        Self {
            bytes: BufferBytes::shared(bytes),
            owner: None,
            readonly: true,
            reallocatable: false,
        }
    }

    fn with_opaque_release_readonly<T, F>(bytes: Arc<[u8]>, opaque: T, on_release: F) -> Self
    where
        T: Any + Send + Sync + 'static,
        F: FnOnce(T) + Send + Sync + 'static,
    {
        Self {
            bytes: BufferBytes::shared(bytes),
            owner: Some(BufferOwner::Opaque(Box::new(TypedOpaqueOwner {
                opaque,
                on_release,
            }))),
            readonly: true,
            reallocatable: false,
        }
    }

    fn with_opaque<T>(bytes: Vec<u8>, opaque: T, readonly: bool) -> Self
    where
        T: Any + Send + Sync + 'static,
    {
        Self {
            bytes: BufferBytes::owned(bytes),
            owner: Some(BufferOwner::Opaque(Box::new(TypedOpaqueOwner {
                opaque,
                on_release: |_opaque| {},
            }))),
            readonly,
            reallocatable: false,
        }
    }

    fn with_opaque_data_release<T, F>(
        bytes: Vec<u8>,
        opaque: T,
        on_release: F,
        readonly: bool,
    ) -> Self
    where
        T: Any + Send + Sync + 'static,
        F: FnOnce(T, Vec<u8>) + Send + Sync + 'static,
    {
        Self {
            bytes: BufferBytes::owned(bytes),
            owner: Some(BufferOwner::OpaqueData(Box::new(TypedOpaqueDataOwner {
                opaque,
                on_release,
            }))),
            readonly,
            reallocatable: false,
        }
    }

    fn with_pool(allocation: BufferPoolAllocation, pool: &Arc<BufferPoolInner>) -> Self {
        let BufferPoolAllocation {
            bytes,
            opaque,
            readonly,
            visible_offset: _,
            visible_len: _,
            allow_oversized_recycle,
        } = allocation;
        Self {
            bytes: BufferBytes::owned(bytes),
            owner: Some(BufferOwner::Pool {
                pool: Arc::clone(pool),
                allocated_len: pool.allocated_len,
                opaque,
                allow_oversized_recycle,
            }),
            readonly,
            reallocatable: false,
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
            bytes: BufferBytes::owned(bytes),
            owner: Some(BufferOwner::Callback(Arc::new(on_release))),
            readonly,
            reallocatable: false,
        }
    }

    fn len(&self) -> usize {
        self.bytes.len()
    }

    fn opaque_ref<T: 'static>(&self) -> Option<&T> {
        match &self.owner {
            Some(BufferOwner::Opaque(owner)) => owner.as_any().downcast_ref::<T>(),
            Some(BufferOwner::OpaqueData(owner)) => owner.as_any().downcast_ref::<T>(),
            _ => None,
        }
    }

    fn pool_opaque_ref<T: 'static>(&self) -> Option<&T> {
        match &self.owner {
            Some(BufferOwner::Pool {
                opaque: Some(opaque),
                ..
            }) => opaque.downcast_ref::<T>(),
            _ => None,
        }
    }

    fn into_pool_allocation(mut self) -> BufferPoolAllocation {
        let opaque = match self.owner.take() {
            Some(BufferOwner::Pool {
                opaque,
                allow_oversized_recycle,
                ..
            }) => {
                self.readonly = false;
                return BufferPoolAllocation {
                    bytes: self.bytes.take_vec(),
                    opaque,
                    readonly: false,
                    visible_offset: 0,
                    visible_len: None,
                    allow_oversized_recycle,
                };
            }
            _ => None,
        };
        self.readonly = false;
        BufferPoolAllocation {
            bytes: self.bytes.take_vec(),
            opaque,
            readonly: false,
            visible_offset: 0,
            visible_len: None,
            allow_oversized_recycle: false,
        }
    }
}

enum BufferBytes {
    Owned(Vec<u8>),
    Static(&'static [u8]),
    Shared(Arc<[u8]>),
}

impl BufferBytes {
    fn owned(bytes: Vec<u8>) -> Self {
        Self::Owned(bytes)
    }

    fn static_slice(bytes: &'static [u8]) -> Self {
        Self::Static(bytes)
    }

    fn shared(bytes: Arc<[u8]>) -> Self {
        Self::Shared(bytes)
    }

    fn as_slice(&self) -> &[u8] {
        match self {
            Self::Owned(bytes) => bytes.as_slice(),
            Self::Static(bytes) => bytes,
            Self::Shared(bytes) => bytes,
        }
    }

    fn as_mut_vec(&mut self) -> Option<&mut Vec<u8>> {
        match self {
            Self::Owned(bytes) => Some(bytes),
            Self::Static(_) | Self::Shared(_) => None,
        }
    }

    fn is_owned(&self) -> bool {
        matches!(self, Self::Owned(_))
    }

    fn len(&self) -> usize {
        self.as_slice().len()
    }

    fn take_vec(&mut self) -> Vec<u8> {
        match std::mem::replace(self, Self::Owned(Vec::new())) {
            Self::Owned(bytes) => bytes,
            Self::Static(bytes) => bytes.to_vec(),
            Self::Shared(bytes) => bytes.to_vec(),
        }
    }
}

impl std::fmt::Debug for BufferBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owned(bytes) => f
                .debug_struct("BufferBytes")
                .field("kind", &"owned")
                .field("bytes", bytes)
                .finish(),
            Self::Static(bytes) => f
                .debug_struct("BufferBytes")
                .field("kind", &"static")
                .field("bytes", bytes)
                .finish(),
            Self::Shared(bytes) => f
                .debug_struct("BufferBytes")
                .field("kind", &"shared")
                .field("bytes", bytes)
                .finish(),
        }
    }
}

impl std::fmt::Debug for BufferStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let owner = match self.owner {
            Some(BufferOwner::Pool { .. }) => "pool",
            Some(BufferOwner::Callback(_)) => "callback",
            Some(BufferOwner::Opaque(_)) => "opaque",
            Some(BufferOwner::OpaqueData(_)) => "opaque-data",
            None => "none",
        };
        f.debug_struct("BufferStorage")
            .field("bytes", &self.bytes)
            .field("owner", &owner)
            .field("readonly", &self.readonly)
            .field("reallocatable", &self.reallocatable)
            .finish()
    }
}

impl Clone for BufferStorage {
    fn clone(&self) -> Self {
        Self::new(self.bytes.as_slice().to_vec())
    }
}

impl PartialEq for BufferStorage {
    fn eq(&self, other: &Self) -> bool {
        self.bytes.as_slice() == other.bytes.as_slice()
    }
}

impl Eq for BufferStorage {}

impl Drop for BufferStorage {
    fn drop(&mut self) {
        match self.owner.take() {
            Some(BufferOwner::Pool {
                pool,
                allocated_len,
                opaque,
                allow_oversized_recycle,
            }) => {
                let storage = self.bytes.take_vec();
                let storage_len = storage.len();
                let allocation = BufferPoolAllocation {
                    bytes: storage,
                    opaque,
                    readonly: false,
                    visible_offset: 0,
                    visible_len: None,
                    allow_oversized_recycle,
                };
                let shape_matches = if allow_oversized_recycle {
                    storage_len >= allocated_len
                } else {
                    storage_len == allocated_len
                };
                if !shape_matches {
                    pool.callbacks.release(allocation);
                    return;
                };
                match pool.spare.lock() {
                    Ok(mut spare) => spare.push(allocation),
                    Err(_) => pool.callbacks.release(allocation),
                };
            }
            Some(BufferOwner::Callback(on_release)) => {
                let storage = self.bytes.take_vec();
                on_release(storage);
            }
            Some(BufferOwner::Opaque(owner)) => {
                owner.release();
            }
            Some(BufferOwner::OpaqueData(owner)) => {
                let storage = self.bytes.take_vec();
                owner.release(storage);
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
    spare: Mutex<Vec<BufferPoolAllocation>>,
}

impl Drop for BufferPoolInner {
    fn drop(&mut self) {
        let spare = match self.spare.get_mut() {
            Ok(spare) => spare,
            Err(poisoned) => poisoned.into_inner(),
        };
        while let Some(storage) = spare.pop() {
            self.callbacks.release(storage);
        }
        self.callbacks.free_pool();
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

    pub fn uninit(pool: &mut Option<Self>) {
        *pool = None;
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
            Some(storage) => {
                let offset = storage.visible_offset();
                let len = storage.visible_len().unwrap_or_else(|| self.len());
                Ok(BufferRef {
                    data: Arc::new(BufferStorage::with_pool(storage, &self.inner)),
                    offset,
                    len,
                })
            }
            None => {
                let storage = self.allocate_storage()?;
                let offset = storage.visible_offset();
                let len = storage.visible_len().unwrap_or_else(|| self.len());
                Ok(BufferRef {
                    data: Arc::new(BufferStorage::with_pool(storage, &self.inner)),
                    offset,
                    len,
                })
            }
        }
    }

    pub fn recycle(&self, buffer: BufferRef) -> AvResult<()> {
        let BufferRef { data, offset, len } = buffer;
        if offset != 0 || len != self.len() || data.len() != self.allocated_len() {
            return Err(AvError::invalid_argument(format!(
                "buffer shape offset {offset} len {len}/{} does not match pool shape {}/{}",
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
        let allocation = storage.into_pool_allocation();
        self.lock_spare()?.push(allocation);
        Ok(())
    }

    fn allocate_storage(&self) -> AvResult<BufferPoolAllocation> {
        let storage = (self.inner.callbacks.allocate)(self.allocated_len())?;
        if storage.visible_len().is_some() || storage.visible_offset() != 0 {
            let visible_len = storage.visible_len().unwrap_or_else(|| self.len());
            BufferPoolAllocation::validate_visible_range(
                storage.len(),
                storage.visible_offset(),
                visible_len,
            )?;
            return Ok(storage);
        }
        if storage.len() != self.allocated_len() {
            let actual_len = storage.len();
            self.inner.callbacks.release(storage);
            return Err(AvError::invalid_argument(format!(
                "buffer pool allocator returned {actual_len} bytes for {} byte shape",
                self.allocated_len()
            )));
        }
        Ok(storage)
    }

    fn lock_spare(&self) -> AvResult<std::sync::MutexGuard<'_, Vec<BufferPoolAllocation>>> {
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
        allocation_error(format!(
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
        allocation_error(format!(
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

fn allocation_error(message: impl Into<String>) -> AvError {
    AvError::with_code(AvErrorKind::External, AvErrorCode::ENOMEM, message)
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
        &self.data.bytes.as_slice()[self.offset..self.offset + self.len]
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.as_slice().as_ptr()
    }

    pub fn offset(&self) -> usize {
        self.offset
    }

    pub fn strong_count(&self) -> usize {
        Arc::strong_count(&self.data)
    }

    pub fn shares_storage(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.data, &other.data)
    }

    pub fn shares_storage_with_buffer(&self, buffer: &BufferRef) -> bool {
        Arc::ptr_eq(&self.data, &buffer.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AvErrorCode, AvErrorKind};

    #[test]
    fn buffer_ref_public_abi_and_flags_match_pinned_default_native_profile() {
        assert_eq!(AV_BUFFER_REF_ABI_LAYOUT.name, "AVBufferRef");
        assert_eq!(AV_BUFFER_REF_ABI_LAYOUT.size, 24);
        assert_eq!(AV_BUFFER_REF_ABI_LAYOUT.align, 8);
        assert_eq!(
            AV_BUFFER_REF_ABI_LAYOUT.fields,
            &[
                BufferAbiField {
                    name: "buffer",
                    offset: 0,
                    size: 8,
                },
                BufferAbiField {
                    name: "data",
                    offset: 8,
                    size: 8,
                },
                BufferAbiField {
                    name: "size",
                    offset: 16,
                    size: 8,
                },
            ]
        );

        assert!(AV_BUFFER_REF_ABI_LAYOUT.align.is_power_of_two());
        for field in AV_BUFFER_REF_ABI_LAYOUT.fields {
            assert!(
                field.offset + field.size <= AV_BUFFER_REF_ABI_LAYOUT.size,
                "{}.{} extends beyond the pinned struct size",
                AV_BUFFER_REF_ABI_LAYOUT.name,
                field.name
            );
        }

        assert_eq!(AV_BUFFER_FLAG_READONLY, 1);
    }

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
    fn callback_owned_buffer_with_opaque_releases_visible_bytes_and_full_storage() {
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let capture = std::sync::Arc::clone(&released);

        {
            let mut buffer = BufferRef::from_vec_with_len_and_opaque_release_callback(
                vec![10, 11, 12, 13],
                3,
                901usize,
                move |opaque, bytes| {
                    capture.lock().unwrap().push((opaque, bytes));
                },
            )
            .unwrap();

            assert_eq!(buffer.len(), 3);
            assert_eq!(buffer.allocated_len(), 4);
            assert_eq!(buffer.as_slice(), &[10, 11, 12]);
            assert_eq!(buffer.padding_slice(), &[13]);
            assert_eq!(buffer.opaque_ref::<usize>().copied(), Some(901));
            buffer.make_mut()[1] = 42;
            assert_eq!(buffer.as_slice(), &[10, 42, 12]);
        }

        assert_eq!(*released.lock().unwrap(), vec![(901, vec![10, 42, 12, 13])]);
    }

    #[test]
    fn callback_owned_buffer_with_opaque_rejects_invalid_visible_len_without_release() {
        let released = std::sync::Arc::new(std::sync::Mutex::new(0usize));
        let capture = std::sync::Arc::clone(&released);
        assert_eq!(
            BufferRef::from_vec_with_len_and_opaque_release_callback(
                vec![1],
                2,
                902usize,
                move |_opaque, _bytes| {
                    *capture.lock().unwrap() += 1;
                },
            )
            .unwrap_err()
            .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(*released.lock().unwrap(), 0);
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
    fn static_readonly_buffer_borrows_and_detaches_on_mutation() {
        static STORAGE: &[u8] = &[1, 2, 3, 0];
        let mut buffer = BufferRef::from_static_slice_with_len_readonly(STORAGE, 3).unwrap();
        let shared = buffer.clone();

        assert_eq!(buffer.as_slice(), &[1, 2, 3]);
        assert_eq!(buffer.padding_slice(), &[0]);
        assert!(std::ptr::eq(
            buffer.as_padded_slice().as_ptr(),
            STORAGE.as_ptr()
        ));
        assert!(buffer.is_readonly());
        assert!(!buffer.is_writable());
        assert!(buffer.get_mut().is_none());

        buffer.make_mut()[1] = 9;

        assert_eq!(buffer.as_slice(), &[1, 9, 3]);
        assert_eq!(buffer.padding_slice(), &[0]);
        assert!(!std::ptr::eq(
            buffer.as_padded_slice().as_ptr(),
            STORAGE.as_ptr()
        ));
        assert!(!buffer.is_readonly());
        assert!(buffer.is_writable());
        assert_eq!(shared.as_slice(), &[1, 2, 3]);
        assert!(std::ptr::eq(
            shared.as_padded_slice().as_ptr(),
            STORAGE.as_ptr()
        ));
        assert!(shared.is_readonly());
    }

    #[test]
    fn static_readonly_buffer_resize_detaches_and_zeroes_padding() {
        static STORAGE: &[u8] = &[4, 5, 6, 7];
        let mut buffer = BufferRef::from_static_slice_readonly(STORAGE);

        buffer.resize_with_padding(2, 2).unwrap();

        assert_eq!(buffer.as_slice(), &[4, 5]);
        assert_eq!(buffer.padding_slice(), &[0, 0]);
        assert!(!std::ptr::eq(
            buffer.as_padded_slice().as_ptr(),
            STORAGE.as_ptr()
        ));
        assert!(!buffer.is_readonly());
        assert!(buffer.is_writable());
        assert_eq!(STORAGE, &[4, 5, 6, 7]);
    }

    #[test]
    fn static_readonly_buffer_rejects_invalid_visible_len() {
        static STORAGE: &[u8] = &[1, 2];

        assert_eq!(
            BufferRef::from_static_slice_with_len_readonly(STORAGE, 3)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
    }

    #[test]
    fn shared_readonly_buffer_keeps_arc_storage_until_detach() {
        let storage: Arc<[u8]> = vec![1, 2, 3, 0].into();
        let mut buffer =
            BufferRef::from_shared_slice_with_len_readonly(Arc::clone(&storage), 3).unwrap();
        let shared = buffer.clone();

        assert_eq!(Arc::strong_count(&storage), 2);
        assert_eq!(buffer.as_slice(), &[1, 2, 3]);
        assert_eq!(buffer.padding_slice(), &[0]);
        assert!(std::ptr::eq(
            buffer.as_padded_slice().as_ptr(),
            storage.as_ptr()
        ));
        assert!(buffer.is_readonly());
        assert!(!buffer.is_writable());
        assert!(buffer.get_mut().is_none());

        buffer.make_mut()[0] = 9;

        assert_eq!(buffer.as_slice(), &[9, 2, 3]);
        assert_eq!(buffer.padding_slice(), &[0]);
        assert!(!std::ptr::eq(
            buffer.as_padded_slice().as_ptr(),
            storage.as_ptr()
        ));
        assert!(!buffer.is_readonly());
        assert!(buffer.is_writable());
        assert_eq!(shared.as_slice(), &[1, 2, 3]);
        assert!(shared.is_readonly());
        assert!(std::ptr::eq(
            shared.as_padded_slice().as_ptr(),
            storage.as_ptr()
        ));
        assert_eq!(Arc::strong_count(&storage), 2);
        drop(shared);
        assert_eq!(Arc::strong_count(&storage), 1);
    }

    #[test]
    fn shared_readonly_buffer_resize_detaches_without_mutating_source() {
        let storage: Arc<[u8]> = vec![4, 5, 6].into();
        let mut buffer = BufferRef::from_shared_slice_readonly(Arc::clone(&storage));

        buffer.resize_with_padding(5, 1).unwrap();

        assert_eq!(buffer.as_slice(), &[4, 5, 6, 0, 0]);
        assert_eq!(buffer.padding_slice(), &[0]);
        assert!(!std::ptr::eq(
            buffer.as_padded_slice().as_ptr(),
            storage.as_ptr()
        ));
        assert!(!buffer.is_readonly());
        assert!(buffer.is_writable());
        assert_eq!(storage.as_ref(), &[4, 5, 6]);
        assert_eq!(Arc::strong_count(&storage), 1);
    }

    #[test]
    fn shared_readonly_buffer_rejects_invalid_visible_len_without_taking_arc() {
        let storage: Arc<[u8]> = vec![1, 2].into();

        assert_eq!(
            BufferRef::from_shared_slice_with_len_readonly(Arc::clone(&storage), 3)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(Arc::strong_count(&storage), 1);
    }

    #[test]
    fn external_readonly_buffer_releases_opaque_after_last_original_reference() {
        let storage: Arc<[u8]> = vec![7, 8, 9, 0].into();
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));
        let capture = std::sync::Arc::clone(&released);
        let buffer = BufferRef::from_external_slice_with_len_and_opaque_readonly(
            Arc::clone(&storage),
            3,
            42usize,
            move |opaque| {
                capture.lock().unwrap().push(opaque);
            },
        )
        .unwrap();
        let slice = buffer.slice(1, 2).unwrap();
        let shared = buffer.clone();

        assert_eq!(buffer.as_slice(), &[7, 8, 9]);
        assert_eq!(buffer.padding_slice(), &[0]);
        assert!(buffer.is_readonly());
        assert!(!buffer.is_writable());
        assert!(std::ptr::eq(
            buffer.as_padded_slice().as_ptr(),
            storage.as_ptr()
        ));
        assert_eq!(Arc::strong_count(&storage), 2);

        drop(buffer);
        assert!(released.lock().unwrap().is_empty());
        drop(slice);
        assert!(released.lock().unwrap().is_empty());
        drop(shared);

        assert_eq!(*released.lock().unwrap(), vec![42]);
        assert_eq!(Arc::strong_count(&storage), 1);
    }

    #[test]
    fn external_readonly_buffer_detach_releases_unique_opaque() {
        let storage: Arc<[u8]> = vec![1, 2, 3, 0].into();
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<&'static str>::new()));
        let capture = std::sync::Arc::clone(&released);
        let mut buffer = BufferRef::from_external_slice_with_len_and_opaque_readonly(
            Arc::clone(&storage),
            3,
            "external-owner",
            move |opaque| {
                capture.lock().unwrap().push(opaque);
            },
        )
        .unwrap();

        assert_eq!(buffer.opaque_ref::<&'static str>(), Some(&"external-owner"));
        assert!(buffer.opaque_ref::<usize>().is_none());

        buffer.make_mut()[2] = 8;

        assert_eq!(*released.lock().unwrap(), vec!["external-owner"]);
        assert_eq!(buffer.as_slice(), &[1, 2, 8]);
        assert_eq!(buffer.padding_slice(), &[0]);
        assert!(!buffer.is_readonly());
        assert!(buffer.is_writable());
        assert!(buffer.opaque_ref::<&'static str>().is_none());
        assert!(!std::ptr::eq(
            buffer.as_padded_slice().as_ptr(),
            storage.as_ptr()
        ));
        assert_eq!(Arc::strong_count(&storage), 1);
    }

    #[test]
    fn external_readonly_buffer_exposes_typed_opaque_until_original_release() {
        #[derive(Debug, PartialEq, Eq)]
        struct Token {
            id: usize,
        }

        let storage: Arc<[u8]> = vec![9, 8, 7].into();
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));
        let capture = std::sync::Arc::clone(&released);
        let mut buffer = BufferRef::from_external_slice_with_opaque_readonly(
            Arc::clone(&storage),
            Token { id: 11 },
            move |opaque| {
                capture.lock().unwrap().push(opaque.id);
            },
        );
        let shared = buffer.clone();

        assert_eq!(buffer.opaque_ref::<Token>().map(|token| token.id), Some(11));
        assert!(buffer.opaque_ref::<usize>().is_none());
        assert_eq!(shared.opaque_ref::<Token>().map(|token| token.id), Some(11));

        buffer.make_mut()[0] = 3;

        assert!(released.lock().unwrap().is_empty());
        assert!(buffer.opaque_ref::<Token>().is_none());
        assert_eq!(shared.opaque_ref::<Token>().map(|token| token.id), Some(11));
        drop(buffer);
        assert!(released.lock().unwrap().is_empty());
        drop(shared);
        assert_eq!(*released.lock().unwrap(), vec![11]);
        assert_eq!(Arc::strong_count(&storage), 1);
    }

    #[test]
    fn external_readonly_buffer_resize_waits_for_shared_original() {
        let storage: Arc<[u8]> = vec![4, 5, 6].into();
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));
        let capture = std::sync::Arc::clone(&released);
        let mut buffer = BufferRef::from_external_slice_with_opaque_readonly(
            Arc::clone(&storage),
            77usize,
            move |opaque| {
                capture.lock().unwrap().push(opaque);
            },
        );
        let shared = buffer.clone();

        assert_eq!(buffer.opaque_ref::<usize>(), Some(&77));
        assert_eq!(shared.opaque_ref::<usize>(), Some(&77));
        buffer.resize_with_padding(5, 1).unwrap();

        assert!(released.lock().unwrap().is_empty());
        assert_eq!(buffer.as_slice(), &[4, 5, 6, 0, 0]);
        assert_eq!(buffer.padding_slice(), &[0]);
        assert!(!buffer.is_readonly());
        assert!(buffer.is_writable());
        assert!(buffer.opaque_ref::<usize>().is_none());
        assert_eq!(shared.as_slice(), &[4, 5, 6]);
        assert!(shared.is_readonly());
        assert_eq!(shared.opaque_ref::<usize>(), Some(&77));
        assert!(std::ptr::eq(
            shared.as_padded_slice().as_ptr(),
            storage.as_ptr()
        ));
        drop(buffer);
        assert!(released.lock().unwrap().is_empty());
        drop(shared);
        assert_eq!(*released.lock().unwrap(), vec![77]);
        assert_eq!(Arc::strong_count(&storage), 1);
    }

    #[test]
    fn external_readonly_buffer_rejects_invalid_visible_len_without_release() {
        let storage: Arc<[u8]> = vec![1, 2].into();
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));
        let capture = std::sync::Arc::clone(&released);

        assert_eq!(
            BufferRef::from_external_slice_with_len_and_opaque_readonly(
                Arc::clone(&storage),
                3,
                5usize,
                move |opaque| {
                    capture.lock().unwrap().push(opaque);
                },
            )
            .unwrap_err()
            .kind(),
            AvErrorKind::InvalidArgument
        );
        assert!(released.lock().unwrap().is_empty());
        assert_eq!(Arc::strong_count(&storage), 1);
    }

    #[test]
    fn opaque_data_buffers_preserve_owner_until_original_release() {
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let capture = std::sync::Arc::clone(&released);
        let mut unique = BufferRef::from_vec_with_opaque_release_callback(
            vec![1, 2, 3],
            17usize,
            move |opaque, bytes| {
                capture.lock().unwrap().push((opaque, bytes));
            },
        );

        assert!(unique.is_writable());
        assert!(!unique.is_readonly());
        assert_eq!(unique.opaque_ref::<usize>(), Some(&17));
        unique.make_mut()[1] = 9;
        assert_eq!(unique.opaque_ref::<usize>(), Some(&17));
        drop(unique);
        assert_eq!(*released.lock().unwrap(), vec![(17, vec![1, 9, 3])]);

        let shared_released =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let shared_capture = std::sync::Arc::clone(&shared_released);
        let source = BufferRef::from_vec_with_opaque_release_callback(
            vec![4, 5, 6],
            23usize,
            move |opaque, bytes| {
                shared_capture.lock().unwrap().push((opaque, bytes));
            },
        );
        let mut detached = source.clone();
        assert_eq!(detached.opaque_ref::<usize>(), Some(&23));
        assert!(detached.shares_storage(&source));
        assert_eq!(source.strong_count(), 2);
        assert!(!source.is_writable());
        assert!(!detached.is_writable());

        detached.make_mut()[0] = 8;

        assert_eq!(source.opaque_ref::<usize>(), Some(&23));
        assert!(detached.opaque_ref::<usize>().is_none());
        assert!(!detached.shares_storage(&source));
        assert_eq!(detached.as_slice(), &[8, 5, 6]);
        assert_eq!(source.as_slice(), &[4, 5, 6]);
        drop(detached);
        assert!(shared_released.lock().unwrap().is_empty());
        drop(source);
        assert_eq!(*shared_released.lock().unwrap(), vec![(23, vec![4, 5, 6])]);
    }

    #[test]
    fn default_free_opaque_data_buffers_preserve_opaque_until_detach() {
        let mut unique = BufferRef::from_vec_with_opaque(vec![1, 2, 3], 17usize);
        assert!(unique.is_writable());
        assert!(!unique.is_readonly());
        assert_eq!(unique.opaque_ref::<usize>(), Some(&17));
        unique.make_mut()[1] = 9;
        assert_eq!(unique.as_slice(), &[1, 9, 3]);
        assert_eq!(unique.opaque_ref::<usize>(), Some(&17));

        let source = BufferRef::from_vec_with_opaque(vec![4, 5, 6], 23usize);
        let mut detached = BufferRef::ref_from(&source);
        assert_eq!(source.opaque_ref::<usize>(), Some(&23));
        assert_eq!(detached.opaque_ref::<usize>(), Some(&23));
        assert!(detached.shares_storage(&source));
        assert_eq!(source.strong_count(), 2);
        assert!(!source.is_writable());
        assert!(!detached.is_writable());

        detached.make_mut()[0] = 8;

        assert_eq!(source.as_slice(), &[4, 5, 6]);
        assert_eq!(source.opaque_ref::<usize>(), Some(&23));
        assert_eq!(detached.as_slice(), &[8, 5, 6]);
        assert!(detached.opaque_ref::<usize>().is_none());
        assert!(!detached.shares_storage(&source));
        assert_eq!(source.strong_count(), 1);
        assert!(source.is_writable());

        let mut readonly = BufferRef::from_vec_with_opaque_readonly(vec![7, 8, 9], 31usize);
        assert!(readonly.is_readonly());
        assert!(!readonly.is_writable());
        assert_eq!(readonly.opaque_ref::<usize>(), Some(&31));
        readonly.make_mut()[2] = 10;
        assert_eq!(readonly.as_slice(), &[7, 8, 10]);
        assert!(!readonly.is_readonly());
        assert!(readonly.is_writable());
        assert!(readonly.opaque_ref::<usize>().is_none());

        let mut realloc = Some(BufferRef::from_vec_with_opaque(vec![10, 11, 12], 41usize));
        let before = realloc.as_ref().unwrap().as_ptr();
        BufferRef::realloc(&mut realloc, 5).unwrap();
        let realloc = realloc.expect("realloc keeps destination");
        assert_eq!(realloc.len(), 5);
        assert_eq!(&realloc.as_slice()[..3], &[10, 11, 12]);
        assert_eq!(&realloc.as_slice()[3..], &[0, 0]);
        assert!(realloc.is_writable());
        assert!(realloc.opaque_ref::<usize>().is_none());
        assert!(!std::ptr::eq(before, realloc.as_ptr()));
    }

    #[test]
    fn zero_length_opaque_data_buffer_preserves_owner_until_release() {
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let capture = std::sync::Arc::clone(&released);
        let zero = BufferRef::from_vec_with_opaque_release_callback(
            Vec::new(),
            321usize,
            move |opaque, bytes| {
                capture.lock().unwrap().push((opaque, bytes));
            },
        );

        assert_eq!(zero.len(), 0);
        assert!(zero.as_slice().is_empty());
        assert_eq!(zero.allocated_len(), 0);
        assert!(zero.is_writable());
        assert!(!zero.is_readonly());
        assert_eq!(zero.strong_count(), 1);
        assert_eq!(zero.opaque_ref::<usize>(), Some(&321));
        assert!(released.lock().unwrap().is_empty());

        drop(zero);
        assert_eq!(*released.lock().unwrap(), vec![(321, Vec::new())]);
    }

    #[test]
    fn zero_length_readonly_opaque_data_buffer_detaches_and_releases_owner() {
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let capture = std::sync::Arc::clone(&released);
        let mut buffer = BufferRef::from_vec_with_opaque_release_callback_readonly(
            Vec::new(),
            987usize,
            move |opaque, bytes| {
                capture.lock().unwrap().push((opaque, bytes));
            },
        );

        assert_eq!(buffer.len(), 0);
        assert!(buffer.as_slice().is_empty());
        assert_eq!(buffer.allocated_len(), 0);
        assert!(buffer.is_readonly());
        assert!(!buffer.is_writable());
        assert_eq!(buffer.strong_count(), 1);
        assert_eq!(buffer.opaque_ref::<usize>(), Some(&987));
        assert!(released.lock().unwrap().is_empty());

        assert!(buffer.make_mut().is_empty());

        assert_eq!(*released.lock().unwrap(), vec![(987, Vec::new())]);
        assert_eq!(buffer.len(), 0);
        assert!(buffer.as_slice().is_empty());
        assert_eq!(buffer.allocated_len(), 0);
        assert!(!buffer.is_readonly());
        assert!(buffer.is_writable());
        assert!(buffer.opaque_ref::<usize>().is_none());
    }

    #[test]
    fn readonly_opaque_data_buffers_release_original_bytes_on_detach() {
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let capture = std::sync::Arc::clone(&released);
        let mut buffer = BufferRef::from_vec_with_len_and_opaque_release_callback_readonly(
            vec![7, 8, 9, 0],
            3,
            31usize,
            move |opaque, bytes| {
                capture.lock().unwrap().push((opaque, bytes));
            },
        )
        .unwrap();

        assert!(buffer.is_readonly());
        assert!(!buffer.is_writable());
        assert_eq!(buffer.opaque_ref::<usize>(), Some(&31));
        buffer.make_mut()[2] = 4;

        assert_eq!(*released.lock().unwrap(), vec![(31, vec![7, 8, 9, 0])]);
        assert!(!buffer.is_readonly());
        assert!(buffer.is_writable());
        assert!(buffer.opaque_ref::<usize>().is_none());
        assert_eq!(buffer.as_slice(), &[7, 8, 4]);
        assert_eq!(buffer.padding_slice(), &[0]);

        let shared_released =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let shared_capture = std::sync::Arc::clone(&shared_released);
        let shared_source = BufferRef::from_vec_with_opaque_release_callback_readonly(
            vec![10, 11, 12],
            43usize,
            move |opaque, bytes| {
                shared_capture.lock().unwrap().push((opaque, bytes));
            },
        );
        let mut shared_detached = BufferRef::ref_from(&shared_source);

        assert!(shared_source.shares_storage(&shared_detached));
        assert_eq!(shared_source.strong_count(), 2);
        assert!(shared_source.is_readonly());
        assert!(shared_detached.is_readonly());
        assert!(!shared_source.is_writable());
        assert!(!shared_detached.is_writable());
        assert_eq!(shared_source.opaque_ref::<usize>(), Some(&43));
        assert_eq!(shared_detached.opaque_ref::<usize>(), Some(&43));

        shared_detached.make_mut()[0] = 99;

        assert_eq!(shared_source.as_slice(), &[10, 11, 12]);
        assert_eq!(shared_source.strong_count(), 1);
        assert!(shared_source.is_readonly());
        assert_eq!(shared_source.opaque_ref::<usize>(), Some(&43));
        assert_eq!(shared_detached.as_slice(), &[99, 11, 12]);
        assert!(!shared_detached.is_readonly());
        assert!(shared_detached.is_writable());
        assert!(shared_detached.opaque_ref::<usize>().is_none());
        assert!(!shared_detached.shares_storage(&shared_source));
        assert!(shared_released.lock().unwrap().is_empty());
        drop(shared_detached);
        assert!(shared_released.lock().unwrap().is_empty());
        drop(shared_source);
        assert_eq!(
            *shared_released.lock().unwrap(),
            vec![(43, vec![10, 11, 12])]
        );

        let invalid_released =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let invalid_capture = std::sync::Arc::clone(&invalid_released);
        assert_eq!(
            BufferRef::from_vec_with_len_and_opaque_release_callback(
                vec![1, 2],
                3,
                41usize,
                move |opaque, bytes| {
                    invalid_capture.lock().unwrap().push((opaque, bytes));
                },
            )
            .unwrap_err()
            .kind(),
            AvErrorKind::InvalidArgument
        );
        assert!(invalid_released.lock().unwrap().is_empty());
    }

    #[test]
    fn zeroed_buffer_allocates_requested_zero_bytes() {
        let ordinary_empty = BufferRef::from_vec(Vec::new());
        assert!(ordinary_empty.is_empty());
        assert_eq!(ordinary_empty.as_slice(), &[]);
        assert_eq!(ordinary_empty.as_padded_slice(), &[]);
        assert_eq!(ordinary_empty.allocated_len(), 0);
        assert!(ordinary_empty.is_writable());
        assert_eq!(ordinary_empty.strong_count(), 1);

        let empty = BufferRef::zeroed(0).unwrap();
        assert!(empty.is_empty());
        assert_eq!(empty.as_slice(), &[]);
        assert_eq!(empty.as_padded_slice(), &[]);
        assert_eq!(empty.allocated_len(), 0);
        assert!(empty.is_writable());
        assert_eq!(empty.strong_count(), 1);

        let buffer = BufferRef::zeroed(4).unwrap();
        assert_eq!(buffer.as_slice(), &[0, 0, 0, 0]);
        assert_eq!(buffer.allocated_len(), 4);
        assert_eq!(buffer.padding_len(), 0);

        let huge = BufferRef::zeroed(usize::MAX).unwrap_err();
        assert_eq!(huge.kind(), AvErrorKind::External);
        assert_eq!(huge.code(), Some(AvErrorCode::ENOMEM));
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
    fn zero_length_shared_make_mut_detaches_to_writable_empty_ref() {
        let source = BufferRef::zeroed(0).unwrap();
        let mut detached = BufferRef::ref_from(&source);

        assert_eq!(source.len(), 0);
        assert_eq!(detached.len(), 0);
        assert!(source.shares_storage(&detached));
        assert_eq!(source.strong_count(), 2);
        assert!(!source.is_writable());
        assert!(!detached.is_writable());

        detached.make_mut();

        assert_eq!(source.len(), 0);
        assert_eq!(detached.len(), 0);
        assert!(!source.shares_storage(&detached));
        assert_eq!(source.strong_count(), 1);
        assert_eq!(detached.strong_count(), 1);
        assert!(source.is_writable());
        assert!(detached.is_writable());
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
    fn buffer_storage_identity_tracks_refs_slices_and_detach() {
        let mut buffer = BufferRef::copy_from_slice_with_padding(&[1, 2, 3], 1).unwrap();
        let shared = buffer.clone();
        let slice = buffer.slice(1, 2).unwrap();

        assert!(buffer.shares_storage(&shared));
        assert!(buffer.shares_storage_with_slice(&slice));
        assert!(slice.shares_storage_with_buffer(&buffer));
        assert_eq!(buffer.as_ptr(), buffer.as_slice().as_ptr());
        assert_eq!(buffer.as_padded_ptr(), buffer.as_padded_slice().as_ptr());
        assert_eq!(slice.as_ptr(), buffer.as_ptr().wrapping_add(1));

        buffer.make_mut()[0] = 9;

        assert!(!buffer.shares_storage(&shared));
        assert!(!buffer.shares_storage_with_slice(&slice));
        assert!(slice.shares_storage_with_buffer(&shared));
        assert_eq!(buffer.as_slice(), &[9, 2, 3]);
        assert_eq!(shared.as_slice(), &[1, 2, 3]);
        assert_eq!(slice.as_slice(), &[2, 3]);
    }

    #[test]
    fn buffer_storage_identity_distinguishes_equal_independent_bytes() {
        let first = BufferRef::copy_from_slice(&[1, 2, 3]);
        let second = BufferRef::copy_from_slice(&[1, 2, 3]);
        let first_shared = first.clone();
        let first_slice = first.slice(0, 3).unwrap();
        let second_slice = second.slice(0, 3).unwrap();

        assert_eq!(first, second);
        assert!(!first.shares_storage(&second));
        assert!(first.shares_storage(&first_shared));
        assert!(first.shares_storage_with_slice(&first_slice));
        assert!(first_slice.shares_storage_with_buffer(&first_shared));
        assert!(!first_slice.shares_storage_with_buffer(&second));
        assert!(!first_slice.shares_storage(&second_slice));
    }

    #[test]
    fn buffer_ref_replace_and_unref_handle_nullable_c_api_shape() {
        let source = BufferRef::from_vec(vec![1, 2, 3]);
        let mut empty_dst = None;
        BufferRef::replace(&mut empty_dst, Some(&source));
        let copied = empty_dst.as_ref().unwrap();
        assert!(copied.shares_storage(&source));
        assert_eq!(source.strong_count(), 2);

        let replacement = BufferRef::from_vec(vec![4, 5]);
        let released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
        let release_capture = std::sync::Arc::clone(&released);
        let mut dst = Some(BufferRef::from_vec_with_release_callback(
            vec![9, 9],
            move |storage| {
                release_capture.lock().unwrap().push(storage);
            },
        ));
        BufferRef::replace(&mut dst, Some(&replacement));
        assert_eq!(*released.lock().unwrap(), vec![vec![9, 9]]);
        assert!(dst.as_ref().unwrap().shares_storage(&replacement));

        let same_source = BufferRef::from_vec(vec![7, 8, 9]);
        let mut same_dst = Some(BufferRef::ref_from(&same_source));
        BufferRef::replace(&mut same_dst, Some(&same_source));
        assert!(same_dst.as_ref().unwrap().shares_storage(&same_source));
        assert_eq!(same_source.strong_count(), 2);

        let mut self_dst = Some(BufferRef::from_vec(vec![2, 4, 6]));
        let self_before = self_dst.as_ref().unwrap().as_ptr();
        let self_source = BufferRef::ref_from(self_dst.as_ref().unwrap());
        BufferRef::replace(&mut self_dst, Some(&self_source));
        drop(self_source);
        let self_replaced = self_dst.as_ref().unwrap();
        assert_eq!(self_replaced.as_slice(), &[2, 4, 6]);
        assert!(std::ptr::eq(self_replaced.as_ptr(), self_before));
        assert_eq!(self_replaced.strong_count(), 1);
        assert!(self_replaced.is_writable());

        BufferRef::replace(&mut same_dst, None);
        assert!(same_dst.is_none());
        let mut null_dst = None;
        BufferRef::replace(&mut null_dst, None);
        assert!(null_dst.is_none());
        assert!(empty_dst.is_some());
        BufferRef::unref(&mut empty_dst);
        assert!(empty_dst.is_none());
        BufferRef::unref(&mut empty_dst);
        assert!(empty_dst.is_none());
    }

    #[test]
    fn buffer_ref_realloc_handles_nullable_c_api_shape() {
        let mut empty = None;
        BufferRef::realloc(&mut empty, 3).unwrap();
        let mut allocated = empty.take().expect("realloc null allocates");
        assert_eq!(allocated.len(), 3);
        assert_eq!(allocated.allocated_len(), 3);
        assert!(allocated.is_writable());
        assert_eq!(allocated.strong_count(), 1);

        allocated.make_mut().copy_from_slice(&[4, 5, 6]);
        let reallocatable_storage = std::sync::Arc::as_ptr(&allocated.data);
        let mut existing = Some(allocated);
        BufferRef::realloc(&mut existing, 5).unwrap();
        let grown = existing.as_ref().unwrap();
        assert_eq!(std::sync::Arc::as_ptr(&grown.data), reallocatable_storage);
        assert_eq!(grown.len(), 5);
        assert_eq!(&grown.as_slice()[..3], &[4, 5, 6]);

        BufferRef::realloc(&mut existing, 2).unwrap();
        let shrunk = existing.as_ref().unwrap();
        assert_eq!(std::sync::Arc::as_ptr(&shrunk.data), reallocatable_storage);
        assert_eq!(shrunk.as_slice(), &[4, 5]);

        let mut empty_zero = None;
        BufferRef::realloc(&mut empty_zero, 0).unwrap();
        let empty_zero = empty_zero.expect("zero-size realloc null allocates a ref");
        assert_eq!(empty_zero.len(), 0);
        assert_eq!(empty_zero.allocated_len(), 0);
        assert!(empty_zero.is_writable());
        assert_eq!(empty_zero.strong_count(), 1);
        assert!(empty_zero.data.reallocatable);

        let mut ordinary = Some(BufferRef::from_vec(vec![11, 12, 13]));
        let ordinary_storage = std::sync::Arc::as_ptr(&ordinary.as_ref().unwrap().data);
        BufferRef::realloc(&mut ordinary, 5).unwrap();
        let ordinary = ordinary.expect("ordinary realloc result");
        assert_ne!(std::sync::Arc::as_ptr(&ordinary.data), ordinary_storage);
        assert!(ordinary.data.reallocatable);
        assert_eq!(&ordinary.as_slice()[..3], &[11, 12, 13]);

        let mut ordinary_zero = Some(BufferRef::from_vec(vec![31, 32, 33]));
        BufferRef::realloc(&mut ordinary_zero, 0).unwrap();
        let ordinary_zero = ordinary_zero.expect("ordinary zero-size realloc result");
        assert_eq!(ordinary_zero.len(), 0);
        assert!(ordinary_zero.as_slice().is_empty());
        assert!(ordinary_zero.is_writable());
        assert!(ordinary_zero.data.reallocatable);

        let custom_released =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let custom_capture = std::sync::Arc::clone(&custom_released);
        let mut custom = Some(BufferRef::from_vec_with_opaque_release_callback(
            vec![21, 22, 23],
            321usize,
            move |opaque, storage| {
                custom_capture.lock().unwrap().push((opaque, storage));
            },
        ));
        let custom_storage = std::sync::Arc::as_ptr(&custom.as_ref().unwrap().data);
        BufferRef::realloc(&mut custom, 5).unwrap();
        let custom = custom.expect("custom realloc result");
        assert_ne!(std::sync::Arc::as_ptr(&custom.data), custom_storage);
        assert!(custom.data.reallocatable);
        assert_eq!(&custom.as_slice()[..3], &[21, 22, 23]);
        assert!(custom.opaque_ref::<usize>().is_none());
        assert_eq!(
            *custom_released.lock().unwrap(),
            vec![(321, vec![21, 22, 23])]
        );

        let custom_shrink_released =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let custom_shrink_capture = std::sync::Arc::clone(&custom_shrink_released);
        let mut custom_shrink = Some(BufferRef::from_vec_with_opaque_release_callback(
            vec![24, 25, 26, 27],
            324usize,
            move |opaque, storage| {
                custom_shrink_capture
                    .lock()
                    .unwrap()
                    .push((opaque, storage));
            },
        ));
        let custom_shrink_storage = std::sync::Arc::as_ptr(&custom_shrink.as_ref().unwrap().data);
        BufferRef::realloc(&mut custom_shrink, 2).unwrap();
        let custom_shrink = custom_shrink.expect("custom shrink realloc result");
        assert_ne!(
            std::sync::Arc::as_ptr(&custom_shrink.data),
            custom_shrink_storage
        );
        assert!(custom_shrink.data.reallocatable);
        assert!(custom_shrink.is_writable());
        assert_eq!(custom_shrink.as_slice(), &[24, 25]);
        assert!(custom_shrink.opaque_ref::<usize>().is_none());
        assert_eq!(
            *custom_shrink_released.lock().unwrap(),
            vec![(324, vec![24, 25, 26, 27])]
        );
        drop(custom_shrink);
        assert_eq!(
            *custom_shrink_released.lock().unwrap(),
            vec![(324, vec![24, 25, 26, 27])]
        );

        let same_source = BufferRef::copy_from_slice(&[10, 20, 30]);
        let mut same_shared = Some(BufferRef::ref_from(&same_source));
        let same_ptr = same_shared.as_ref().unwrap().as_ptr();
        BufferRef::realloc(&mut same_shared, same_source.len()).unwrap();
        let same_shared = same_shared.expect("same-size realloc result");
        assert!(same_shared.shares_storage(&same_source));
        assert_eq!(same_shared.as_ptr(), same_ptr);
        assert_eq!(same_source.strong_count(), 2);

        let same_custom_released =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let same_custom_capture = std::sync::Arc::clone(&same_custom_released);
        let mut same_custom = Some(BufferRef::from_vec_with_opaque_release_callback(
            vec![41, 42, 43],
            432usize,
            move |opaque, storage| {
                same_custom_capture.lock().unwrap().push((opaque, storage));
            },
        ));
        let same_custom_storage = std::sync::Arc::as_ptr(&same_custom.as_ref().unwrap().data);
        let same_custom_ptr = same_custom.as_ref().unwrap().as_ptr();
        BufferRef::realloc(&mut same_custom, 3).unwrap();
        let same_custom = same_custom.expect("same-size custom realloc result");
        assert_eq!(
            std::sync::Arc::as_ptr(&same_custom.data),
            same_custom_storage
        );
        assert_eq!(same_custom.as_ptr(), same_custom_ptr);
        assert!(!same_custom.data.reallocatable);
        assert!(same_custom.is_writable());
        assert_eq!(same_custom.opaque_ref::<usize>(), Some(&432));
        assert!(same_custom_released.lock().unwrap().is_empty());
        drop(same_custom);
        assert_eq!(
            *same_custom_released.lock().unwrap(),
            vec![(432, vec![41, 42, 43])]
        );

        let readonly_released = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
        let readonly_capture = std::sync::Arc::clone(&readonly_released);
        let mut same_readonly = Some(BufferRef::from_vec_with_release_callback_readonly(
            vec![4, 5, 6],
            move |storage| {
                readonly_capture.lock().unwrap().push(storage);
            },
        ));
        let readonly_ptr = same_readonly.as_ref().unwrap().as_ptr();
        BufferRef::realloc(&mut same_readonly, 3).unwrap();
        let same_readonly = same_readonly.expect("same-size readonly realloc result");
        assert!(same_readonly.is_readonly());
        assert_eq!(same_readonly.as_ptr(), readonly_ptr);
        assert!(readonly_released.lock().unwrap().is_empty());

        let readonly_opaque_released =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let readonly_opaque_capture = std::sync::Arc::clone(&readonly_opaque_released);
        let mut same_readonly_opaque =
            Some(BufferRef::from_vec_with_opaque_release_callback_readonly(
                vec![51, 52, 53],
                543usize,
                move |opaque, storage| {
                    readonly_opaque_capture
                        .lock()
                        .unwrap()
                        .push((opaque, storage));
                },
            ));
        let readonly_opaque_storage =
            std::sync::Arc::as_ptr(&same_readonly_opaque.as_ref().unwrap().data);
        let readonly_opaque_ptr = same_readonly_opaque.as_ref().unwrap().as_ptr();
        BufferRef::realloc(&mut same_readonly_opaque, 3).unwrap();
        let same_readonly_opaque =
            same_readonly_opaque.expect("same-size readonly opaque realloc result");
        assert_eq!(
            std::sync::Arc::as_ptr(&same_readonly_opaque.data),
            readonly_opaque_storage
        );
        assert_eq!(same_readonly_opaque.as_ptr(), readonly_opaque_ptr);
        assert!(same_readonly_opaque.is_readonly());
        assert!(!same_readonly_opaque.data.reallocatable);
        assert_eq!(same_readonly_opaque.opaque_ref::<usize>(), Some(&543));
        assert!(readonly_opaque_released.lock().unwrap().is_empty());
        drop(same_readonly_opaque);
        assert_eq!(
            *readonly_opaque_released.lock().unwrap(),
            vec![(543, vec![51, 52, 53])]
        );

        let readonly_realloc_released =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let readonly_realloc_capture = std::sync::Arc::clone(&readonly_realloc_released);
        let mut readonly_realloc = Some(BufferRef::from_vec_with_opaque_release_callback_readonly(
            vec![81, 82, 83],
            876usize,
            move |opaque, storage| {
                readonly_realloc_capture
                    .lock()
                    .unwrap()
                    .push((opaque, storage));
            },
        ));
        let readonly_realloc_storage =
            std::sync::Arc::as_ptr(&readonly_realloc.as_ref().unwrap().data);
        BufferRef::realloc(&mut readonly_realloc, 5).unwrap();
        let readonly_realloc = readonly_realloc.expect("readonly realloc result");
        assert_ne!(
            std::sync::Arc::as_ptr(&readonly_realloc.data),
            readonly_realloc_storage
        );
        assert!(readonly_realloc.data.reallocatable);
        assert!(!readonly_realloc.is_readonly());
        assert!(readonly_realloc.is_writable());
        assert_eq!(readonly_realloc.len(), 5);
        assert_eq!(&readonly_realloc.as_slice()[..3], &[81, 82, 83]);
        assert!(readonly_realloc.opaque_ref::<usize>().is_none());
        assert_eq!(
            *readonly_realloc_released.lock().unwrap(),
            vec![(876, vec![81, 82, 83])]
        );
        drop(readonly_realloc);
        assert_eq!(
            *readonly_realloc_released.lock().unwrap(),
            vec![(876, vec![81, 82, 83])]
        );

        let readonly_realloc_shrink_released =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let readonly_realloc_shrink_capture =
            std::sync::Arc::clone(&readonly_realloc_shrink_released);
        let mut readonly_realloc_shrink =
            Some(BufferRef::from_vec_with_opaque_release_callback_readonly(
                vec![84, 85, 86, 87],
                878usize,
                move |opaque, storage| {
                    readonly_realloc_shrink_capture
                        .lock()
                        .unwrap()
                        .push((opaque, storage));
                },
            ));
        let readonly_realloc_shrink_storage =
            std::sync::Arc::as_ptr(&readonly_realloc_shrink.as_ref().unwrap().data);
        BufferRef::realloc(&mut readonly_realloc_shrink, 2).unwrap();
        let readonly_realloc_shrink =
            readonly_realloc_shrink.expect("readonly shrink realloc result");
        assert_ne!(
            std::sync::Arc::as_ptr(&readonly_realloc_shrink.data),
            readonly_realloc_shrink_storage
        );
        assert!(readonly_realloc_shrink.data.reallocatable);
        assert!(!readonly_realloc_shrink.is_readonly());
        assert!(readonly_realloc_shrink.is_writable());
        assert_eq!(readonly_realloc_shrink.as_slice(), &[84, 85]);
        assert!(readonly_realloc_shrink.opaque_ref::<usize>().is_none());
        assert_eq!(
            *readonly_realloc_shrink_released.lock().unwrap(),
            vec![(878, vec![84, 85, 86, 87])]
        );
        drop(readonly_realloc_shrink);
        assert_eq!(
            *readonly_realloc_shrink_released.lock().unwrap(),
            vec![(878, vec![84, 85, 86, 87])]
        );

        let shared_readonly_released =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let shared_readonly_capture = std::sync::Arc::clone(&shared_readonly_released);
        let shared_readonly_source = BufferRef::from_vec_with_opaque_release_callback_readonly(
            vec![61, 62, 63],
            654usize,
            move |opaque, storage| {
                shared_readonly_capture
                    .lock()
                    .unwrap()
                    .push((opaque, storage));
            },
        );
        let mut shared_readonly_realloc = Some(BufferRef::ref_from(&shared_readonly_source));
        BufferRef::realloc(&mut shared_readonly_realloc, 5).unwrap();
        let shared_readonly_realloc =
            shared_readonly_realloc.expect("shared readonly realloc result");
        assert_eq!(shared_readonly_source.as_slice(), &[61, 62, 63]);
        assert_eq!(shared_readonly_source.strong_count(), 1);
        assert!(shared_readonly_source.is_readonly());
        assert_eq!(shared_readonly_source.opaque_ref::<usize>(), Some(&654));
        assert_eq!(shared_readonly_realloc.len(), 5);
        assert_eq!(&shared_readonly_realloc.as_slice()[..3], &[61, 62, 63]);
        assert!(shared_readonly_realloc.is_writable());
        assert!(!shared_readonly_realloc.is_readonly());
        assert!(shared_readonly_realloc.opaque_ref::<usize>().is_none());
        assert!(!shared_readonly_realloc.shares_storage(&shared_readonly_source));
        assert!(shared_readonly_released.lock().unwrap().is_empty());
        drop(shared_readonly_realloc);
        assert!(shared_readonly_released.lock().unwrap().is_empty());
        drop(shared_readonly_source);
        assert_eq!(
            *shared_readonly_released.lock().unwrap(),
            vec![(654, vec![61, 62, 63])]
        );

        let shared_readonly_shrink_released =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let shared_readonly_shrink_capture =
            std::sync::Arc::clone(&shared_readonly_shrink_released);
        let shared_readonly_shrink_source =
            BufferRef::from_vec_with_opaque_release_callback_readonly(
                vec![66, 67, 68, 69],
                656usize,
                move |opaque, storage| {
                    shared_readonly_shrink_capture
                        .lock()
                        .unwrap()
                        .push((opaque, storage));
                },
            );
        let mut shared_readonly_shrink = Some(BufferRef::ref_from(&shared_readonly_shrink_source));
        BufferRef::realloc(&mut shared_readonly_shrink, 2).unwrap();
        let shared_readonly_shrink =
            shared_readonly_shrink.expect("shared readonly shrink realloc result");
        assert_eq!(shared_readonly_shrink_source.as_slice(), &[66, 67, 68, 69]);
        assert_eq!(shared_readonly_shrink_source.strong_count(), 1);
        assert!(shared_readonly_shrink_source.is_readonly());
        assert_eq!(
            shared_readonly_shrink_source.opaque_ref::<usize>(),
            Some(&656)
        );
        assert_eq!(shared_readonly_shrink.as_slice(), &[66, 67]);
        assert!(shared_readonly_shrink.is_writable());
        assert!(!shared_readonly_shrink.is_readonly());
        assert!(shared_readonly_shrink.opaque_ref::<usize>().is_none());
        assert!(!shared_readonly_shrink.shares_storage(&shared_readonly_shrink_source));
        assert!(shared_readonly_shrink_released.lock().unwrap().is_empty());
        drop(shared_readonly_shrink);
        assert!(shared_readonly_shrink_released.lock().unwrap().is_empty());
        drop(shared_readonly_shrink_source);
        assert_eq!(
            *shared_readonly_shrink_released.lock().unwrap(),
            vec![(656, vec![66, 67, 68, 69])]
        );

        let shared_custom_released =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let shared_custom_capture = std::sync::Arc::clone(&shared_custom_released);
        let shared_custom_source = BufferRef::from_vec_with_opaque_release_callback(
            vec![71, 72, 73],
            765usize,
            move |opaque, storage| {
                shared_custom_capture
                    .lock()
                    .unwrap()
                    .push((opaque, storage));
            },
        );
        let mut shared_custom_realloc = Some(BufferRef::ref_from(&shared_custom_source));
        BufferRef::realloc(&mut shared_custom_realloc, 5).unwrap();
        let shared_custom_realloc = shared_custom_realloc.expect("shared custom realloc result");
        assert_eq!(shared_custom_source.as_slice(), &[71, 72, 73]);
        assert_eq!(shared_custom_source.strong_count(), 1);
        assert!(shared_custom_source.is_writable());
        assert_eq!(shared_custom_source.opaque_ref::<usize>(), Some(&765));
        assert_eq!(shared_custom_realloc.len(), 5);
        assert_eq!(&shared_custom_realloc.as_slice()[..3], &[71, 72, 73]);
        assert!(shared_custom_realloc.is_writable());
        assert!(shared_custom_realloc.opaque_ref::<usize>().is_none());
        assert!(!shared_custom_realloc.shares_storage(&shared_custom_source));
        assert!(shared_custom_released.lock().unwrap().is_empty());
        drop(shared_custom_realloc);
        assert!(shared_custom_released.lock().unwrap().is_empty());
        drop(shared_custom_source);
        assert_eq!(
            *shared_custom_released.lock().unwrap(),
            vec![(765, vec![71, 72, 73])]
        );

        let shared_custom_shrink_released =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let shared_custom_shrink_capture = std::sync::Arc::clone(&shared_custom_shrink_released);
        let shared_custom_shrink_source = BufferRef::from_vec_with_opaque_release_callback(
            vec![74, 75, 76, 77],
            767usize,
            move |opaque, storage| {
                shared_custom_shrink_capture
                    .lock()
                    .unwrap()
                    .push((opaque, storage));
            },
        );
        let mut shared_custom_shrink = Some(BufferRef::ref_from(&shared_custom_shrink_source));
        BufferRef::realloc(&mut shared_custom_shrink, 2).unwrap();
        let shared_custom_shrink =
            shared_custom_shrink.expect("shared custom shrink realloc result");
        assert_eq!(shared_custom_shrink_source.as_slice(), &[74, 75, 76, 77]);
        assert_eq!(shared_custom_shrink_source.strong_count(), 1);
        assert!(shared_custom_shrink_source.is_writable());
        assert_eq!(
            shared_custom_shrink_source.opaque_ref::<usize>(),
            Some(&767)
        );
        assert_eq!(shared_custom_shrink.as_slice(), &[74, 75]);
        assert!(shared_custom_shrink.is_writable());
        assert!(shared_custom_shrink.opaque_ref::<usize>().is_none());
        assert!(!shared_custom_shrink.shares_storage(&shared_custom_shrink_source));
        assert!(shared_custom_shrink_released.lock().unwrap().is_empty());
        drop(shared_custom_shrink);
        assert!(shared_custom_shrink_released.lock().unwrap().is_empty());
        drop(shared_custom_shrink_source);
        assert_eq!(
            *shared_custom_shrink_released.lock().unwrap(),
            vec![(767, vec![74, 75, 76, 77])]
        );

        let shared_source = BufferRef::from_vec(vec![7, 8, 9]);
        let mut shared_realloc = Some(shared_source.clone());
        BufferRef::realloc(&mut shared_realloc, 4).unwrap();
        let shared_realloc = shared_realloc.expect("shared realloc result");
        assert_eq!(&shared_realloc.as_slice()[..3], &[7, 8, 9]);
        assert_eq!(shared_source.as_slice(), &[7, 8, 9]);
        assert!(!shared_realloc.shares_storage(&shared_source));

        let mut failed = None;
        let failed_err = BufferRef::realloc(&mut failed, usize::MAX).unwrap_err();
        assert_eq!(failed_err.kind(), AvErrorKind::External);
        assert_eq!(failed_err.code(), Some(AvErrorCode::ENOMEM));
        assert!(failed.is_none());

        let mut failed_existing = Some(BufferRef::from_vec(vec![91, 92, 93]));
        let failed_existing_ptr = failed_existing.as_ref().unwrap().as_ptr();
        let failed_existing_err = BufferRef::realloc(&mut failed_existing, usize::MAX).unwrap_err();
        assert_eq!(failed_existing_err.kind(), AvErrorKind::External);
        assert_eq!(failed_existing_err.code(), Some(AvErrorCode::ENOMEM));
        let failed_existing = failed_existing.expect("failed realloc preserves destination");
        assert_eq!(failed_existing.as_slice(), &[91, 92, 93]);
        assert_eq!(failed_existing.as_ptr(), failed_existing_ptr);
    }

    #[test]
    fn buffer_ref_slices_model_offset_data_and_size_refs() {
        let source = BufferRef::from_vec(vec![10, 11, 12, 13]);
        let offset_ref = source.ref_slice(1, 2).unwrap();
        assert_eq!(offset_ref.offset(), 1);
        assert_eq!(offset_ref.len(), 2);
        assert_eq!(offset_ref.as_slice(), &[11, 12]);
        assert_eq!(offset_ref.as_ptr(), source.as_ptr().wrapping_add(1));
        assert_eq!(offset_ref.allocated_len(), 3);
        assert_eq!(offset_ref.padding_slice(), &[13]);
        assert!(offset_ref.shares_storage(&source));
        assert_eq!(source.strong_count(), 2);
        assert!(!offset_ref.is_writable());

        let offset_clone = BufferRef::ref_from(&offset_ref);
        assert!(offset_clone.shares_storage(&offset_ref));
        assert_eq!(offset_clone.offset(), offset_ref.offset());
        assert_eq!(offset_clone.len(), offset_ref.len());
        assert_eq!(offset_clone.as_ptr(), offset_ref.as_ptr());
        assert_eq!(offset_clone.as_slice(), offset_ref.as_slice());
        assert_eq!(source.strong_count(), 3);
        drop(offset_clone);
        assert_eq!(source.strong_count(), 2);

        let mut detached = offset_ref.clone();
        detached.make_mut()[0] = 99;
        assert_eq!(detached.offset(), 0);
        assert_eq!(detached.as_slice(), &[99, 12]);
        assert_eq!(offset_ref.as_slice(), &[11, 12]);
        assert!(!detached.shares_storage(&offset_ref));

        let mut resized = source.ref_slice(1, 2).unwrap();
        resized.resize(3).unwrap();
        assert_eq!(&resized.as_slice()[..2], &[11, 12]);
        assert_eq!(resized.offset(), 0);
        assert!(resized.is_writable());
        assert_eq!(source.as_slice(), &[10, 11, 12, 13]);

        let unique_offset = BufferRef::from_vec(vec![1, 2, 3, 4])
            .into_ref_slice(2, 2)
            .unwrap();
        assert!(unique_offset.is_writable());
        let mut unique_offset = unique_offset;
        let unique_offset_storage = Arc::as_ptr(&unique_offset.data);
        let unique_offset_ptr = unique_offset.as_ptr();
        unique_offset.make_mut()[0] = 7;
        assert_eq!(Arc::as_ptr(&unique_offset.data), unique_offset_storage);
        assert_eq!(unique_offset.as_ptr(), unique_offset_ptr);
        assert_eq!(unique_offset.offset(), 2);
        assert_eq!(unique_offset.as_slice(), &[7, 4]);

        let realloc_offset_base = BufferRef::from_vec(vec![5, 6, 7, 8]);
        let mut realloc_offset = Some(realloc_offset_base.ref_slice(1, 2).unwrap());
        drop(realloc_offset_base);
        let realloc_offset_storage = Arc::as_ptr(&realloc_offset.as_ref().unwrap().data);
        let realloc_offset_ptr = realloc_offset.as_ref().unwrap().as_ptr();
        BufferRef::realloc(&mut realloc_offset, 3).unwrap();
        let realloc_offset = realloc_offset.expect("unique offset realloc result");
        assert_ne!(Arc::as_ptr(&realloc_offset.data), realloc_offset_storage);
        assert_ne!(realloc_offset.as_ptr(), realloc_offset_ptr);
        assert_eq!(realloc_offset.offset(), 0);
        assert_eq!(&realloc_offset.as_slice()[..2], &[6, 7]);
        assert!(realloc_offset.is_writable());
        assert!(realloc_offset.data.reallocatable);

        assert_eq!(
            source.ref_slice(5, 0).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(
            source.ref_slice(3, 2).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
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
    fn buffer_resize_same_shape_shared_storage_detaches() {
        let mut buffer = BufferRef::from_vec(vec![1, 2, 3]);
        let shared = buffer.clone();

        buffer.resize_with_padding(3, 0).unwrap();

        assert!(!buffer.shares_storage(&shared));
        assert!(buffer.is_writable());
        assert_eq!(buffer.as_slice(), &[1, 2, 3]);
        assert_eq!(shared.as_slice(), &[1, 2, 3]);
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
    fn buffer_pool_allocates_recycles_and_reuses_storage_without_clearing() {
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
        assert_eq!(reused.as_slice(), &[4, 5, 6]);
        assert_eq!(reused.padding_slice(), &[0, 0]);
    }

    #[test]
    fn buffer_pool_default_allocator_reuses_storage_without_opaque() {
        let pool = BufferPool::new(3, 0).unwrap();

        let mut first = pool.get().unwrap();
        assert_eq!(first.len(), 3);
        assert_eq!(first.strong_count(), 1);
        assert!(first.is_writable());
        assert!(first.pool_opaque_ref::<usize>().is_none());
        first.make_mut().copy_from_slice(&[0x21, 0x22, 0x23]);
        drop(first);
        assert_eq!(pool.available_count().unwrap(), 1);

        let reused = pool.get().unwrap();
        assert_eq!(pool.available_count().unwrap(), 0);
        assert_eq!(reused.as_slice(), &[0x21, 0x22, 0x23]);
        assert!(reused.pool_opaque_ref::<usize>().is_none());
    }

    #[test]
    fn buffer_pool_init2_default_allocator_runs_pool_free_without_opaque() {
        let pool_frees = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));
        let pool_free_capture = std::sync::Arc::clone(&pool_frees);
        let pool = BufferPool::with_callbacks(
            2,
            0,
            BufferPoolCallbacks::default().with_pool_free(move || {
                pool_free_capture.lock().unwrap().push(88);
            }),
        )
        .unwrap();

        let mut first = pool.get().unwrap();
        assert_eq!(first.len(), 2);
        assert_eq!(first.strong_count(), 1);
        assert!(first.is_writable());
        assert!(first.pool_opaque_ref::<usize>().is_none());
        first.make_mut().copy_from_slice(&[0x31, 0x32]);
        drop(first);
        assert_eq!(pool.available_count().unwrap(), 1);

        let reused = pool.get().unwrap();
        assert_eq!(reused.as_slice(), &[0x31, 0x32]);
        assert!(reused.pool_opaque_ref::<usize>().is_none());
        drop(reused);
        assert!(pool_frees.lock().unwrap().is_empty());
        drop(pool);

        assert_eq!(*pool_frees.lock().unwrap(), vec![88]);
    }

    #[test]
    fn buffer_pool_uninit_handles_nullable_c_api_shape() {
        let mut empty_pool = None;
        BufferPool::uninit(&mut empty_pool);
        assert!(empty_pool.is_none());

        let pool_frees = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));
        let pool_free_capture = std::sync::Arc::clone(&pool_frees);
        let mut pool = Some(
            BufferPool::with_callbacks(
                2,
                0,
                BufferPoolCallbacks::default().with_pool_free(move || {
                    pool_free_capture.lock().unwrap().push(99);
                }),
            )
            .unwrap(),
        );
        BufferPool::uninit(&mut pool);
        assert!(pool.is_none());
        assert_eq!(*pool_frees.lock().unwrap(), vec![99]);
    }

    #[test]
    fn buffer_pool_zero_size_default_allocator_reuses_empty_buffers() {
        let pool = BufferPool::new(0, 0).unwrap();
        assert_eq!(pool.len(), 0);
        assert!(pool.is_empty());
        assert_eq!(pool.allocated_len(), 0);
        assert_eq!(pool.padding_len(), 0);
        assert_eq!(pool.available_count().unwrap(), 0);

        let first = pool.get().unwrap();
        assert_eq!(first.len(), 0);
        assert!(first.is_empty());
        assert_eq!(first.as_slice(), &[]);
        assert_eq!(first.as_padded_slice(), &[]);
        assert!(first.is_writable());
        assert!(first.pool_opaque_ref::<usize>().is_none());
        drop(first);
        assert_eq!(pool.available_count().unwrap(), 1);

        let reuse = pool.get().unwrap();
        assert_eq!(reuse.len(), 0);
        assert_eq!(reuse.allocated_len(), 0);
        assert!(reuse.is_writable());
        assert!(reuse.pool_opaque_ref::<usize>().is_none());
        drop(reuse);
        assert_eq!(pool.available_count().unwrap(), 1);
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
        assert_eq!(first.as_padded_slice(), &[9, 9, 9, 9, 9]);
        first.make_mut().copy_from_slice(&[1, 2, 3]);
        drop(first);
        assert_eq!(pool.available_count().unwrap(), 1);
        assert!(releases.lock().unwrap().is_empty());

        let second = pool.get().unwrap();
        assert_eq!(*allocations.lock().unwrap(), vec![5]);
        assert_eq!(second.as_padded_slice(), &[1, 2, 3, 9, 9]);
        drop(second);
        drop(pool);

        assert_eq!(*releases.lock().unwrap(), vec![vec![1, 2, 3, 9, 9]]);
    }

    #[test]
    fn legacy_custom_buffer_pool_has_no_pool_opaque_or_owner_free() {
        let allocations = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));
        let releases = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
        let allocate_capture = std::sync::Arc::clone(&allocations);
        let release_capture = std::sync::Arc::clone(&releases);
        let pool = BufferPool::with_callbacks(
            3,
            0,
            BufferPoolCallbacks::new(
                move |allocated_len| {
                    allocate_capture.lock().unwrap().push(allocated_len);
                    Ok(vec![0x51, 0x52, 0x53][..allocated_len].to_vec())
                },
                move |storage| {
                    release_capture.lock().unwrap().push(storage);
                },
            ),
        )
        .unwrap();

        let mut first = pool.get().unwrap();
        assert_eq!(*allocations.lock().unwrap(), vec![3]);
        assert_eq!(first.as_slice(), &[0x51, 0x52, 0x53]);
        assert!(first.is_writable());
        assert!(first.pool_opaque_ref::<usize>().is_none());
        first.make_mut().copy_from_slice(&[0xa1, 0xa2, 0xa3]);
        drop(first);
        assert_eq!(pool.available_count().unwrap(), 1);
        assert!(releases.lock().unwrap().is_empty());

        let reuse = pool.get().unwrap();
        assert_eq!(*allocations.lock().unwrap(), vec![3]);
        assert_eq!(reuse.as_slice(), &[0xa1, 0xa2, 0xa3]);
        assert!(reuse.is_writable());
        assert!(reuse.pool_opaque_ref::<usize>().is_none());
        drop(reuse);
        drop(pool);

        assert_eq!(*releases.lock().unwrap(), vec![vec![0xa1, 0xa2, 0xa3]]);
    }

    #[test]
    fn custom_buffer_pool_reuses_and_releases_multiple_spares_lifo() {
        let next_allocation = std::sync::Arc::new(std::sync::Mutex::new(0u8));
        let releases = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
        let allocate_capture = std::sync::Arc::clone(&next_allocation);
        let release_capture = std::sync::Arc::clone(&releases);
        let pool = BufferPool::with_callbacks(
            2,
            0,
            BufferPoolCallbacks::new(
                move |allocated_len| {
                    let mut next = allocate_capture.lock().unwrap();
                    let base = 0x70 + *next;
                    *next += 1;
                    Ok(vec![base; allocated_len])
                },
                move |storage| {
                    release_capture.lock().unwrap().push(storage);
                },
            ),
        )
        .unwrap();

        let mut first = pool.get().unwrap();
        let mut second = pool.get().unwrap();
        assert_eq!(first.as_slice(), &[0x70, 0x70]);
        assert_eq!(second.as_slice(), &[0x71, 0x71]);
        first.make_mut().copy_from_slice(&[0xa1, 0xa2]);
        second.make_mut().copy_from_slice(&[0xb1, 0xb2]);
        drop(first);
        drop(second);
        assert_eq!(pool.available_count().unwrap(), 2);

        let reuse_first = pool.get().unwrap();
        let reuse_second = pool.get().unwrap();
        assert_eq!(reuse_first.as_slice(), &[0xb1, 0xb2]);
        assert_eq!(reuse_second.as_slice(), &[0xa1, 0xa2]);
        drop(reuse_first);
        drop(reuse_second);
        drop(pool);

        assert_eq!(
            *releases.lock().unwrap(),
            vec![vec![0xa1, 0xa2], vec![0xb1, 0xb2]]
        );
    }

    #[test]
    fn custom_buffer_pool_preserves_buffer_opaque_and_runs_pool_free_last() {
        #[derive(Debug)]
        struct PoolToken {
            id: usize,
        }

        let next_id = std::sync::Arc::new(std::sync::Mutex::new(40usize));
        let releases = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let pool_free_events =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::<&'static str>::new()));
        let next_id_capture = std::sync::Arc::clone(&next_id);
        let release_capture = std::sync::Arc::clone(&releases);
        let pool_free_capture = std::sync::Arc::clone(&pool_free_events);
        let pool = BufferPool::with_callbacks(
            3,
            0,
            BufferPoolCallbacks::with_allocation_callbacks(
                move |allocated_len| {
                    let mut next_id = next_id_capture.lock().unwrap();
                    *next_id += 1;
                    Ok(BufferPoolAllocation::with_opaque(
                        vec![1, 2, 3][..allocated_len].to_vec(),
                        PoolToken { id: *next_id },
                    ))
                },
                move |allocation| {
                    let id = allocation
                        .opaque_ref::<PoolToken>()
                        .map(|token| token.id)
                        .unwrap_or_default();
                    release_capture
                        .lock()
                        .unwrap()
                        .push((id, allocation.into_vec()));
                },
            )
            .with_pool_free(move || {
                pool_free_capture.lock().unwrap().push("pool-free");
            }),
        )
        .unwrap();

        let mut first = pool.get().unwrap();
        assert_eq!(
            first.pool_opaque_ref::<PoolToken>().map(|token| token.id),
            Some(41)
        );
        first.make_mut().copy_from_slice(&[0xaa, 0xbb, 0xcc]);
        drop(first);

        let reused = pool.get().unwrap();
        assert_eq!(
            reused.pool_opaque_ref::<PoolToken>().map(|token| token.id),
            Some(41)
        );
        assert_eq!(reused.as_slice(), &[0xaa, 0xbb, 0xcc]);
        drop(pool);
        assert!(releases.lock().unwrap().is_empty());
        assert!(pool_free_events.lock().unwrap().is_empty());
        drop(reused);

        assert_eq!(
            *releases.lock().unwrap(),
            vec![(41, vec![0xaa, 0xbb, 0xcc])]
        );
        assert_eq!(*pool_free_events.lock().unwrap(), vec!["pool-free"]);
    }

    #[test]
    fn custom_buffer_pool_unique_make_mut_preserves_pool_ownership() {
        #[derive(Debug)]
        struct PoolToken {
            id: usize,
            size: usize,
        }

        let releases = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let pool_frees = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));
        let release_capture = std::sync::Arc::clone(&releases);
        let pool_free_capture = std::sync::Arc::clone(&pool_frees);
        let pool = BufferPool::with_callbacks(
            3,
            0,
            BufferPoolCallbacks::with_allocation_callbacks(
                |allocated_len| {
                    assert_eq!(allocated_len, 3);
                    Ok(BufferPoolAllocation::with_opaque(
                        vec![1, 2, 3],
                        PoolToken {
                            id: 62,
                            size: allocated_len,
                        },
                    ))
                },
                move |allocation| {
                    let token = allocation
                        .opaque_ref::<PoolToken>()
                        .expect("pool token should be preserved");
                    release_capture
                        .lock()
                        .unwrap()
                        .push((token.id, allocation.as_slice().to_vec()));
                },
            )
            .with_pool_free(move || {
                pool_free_capture.lock().unwrap().push(62);
            }),
        )
        .unwrap();

        let mut buffer = pool.get().unwrap();
        buffer.make_mut().copy_from_slice(&[0x62, 0x63, 0x64]);
        let original_ptr = buffer.as_ptr();
        buffer.make_mut();

        assert_eq!(buffer.as_ptr(), original_ptr);
        assert_eq!(buffer.as_slice(), &[0x62, 0x63, 0x64]);
        assert!(buffer.is_writable());
        assert_eq!(
            buffer
                .pool_opaque_ref::<PoolToken>()
                .map(|token| (token.id, token.size)),
            Some((62, 3))
        );
        assert!(releases.lock().unwrap().is_empty());
        assert!(pool_frees.lock().unwrap().is_empty());
        assert_eq!(pool.available_count().unwrap(), 0);

        drop(buffer);
        assert!(releases.lock().unwrap().is_empty());
        assert!(pool_frees.lock().unwrap().is_empty());
        assert_eq!(pool.available_count().unwrap(), 1);

        let reused = pool.get().unwrap();
        assert_eq!(reused.as_slice(), &[0x62, 0x63, 0x64]);
        assert_eq!(
            reused
                .pool_opaque_ref::<PoolToken>()
                .map(|token| (token.id, token.size)),
            Some((62, 3))
        );

        drop(reused);
        drop(pool);
        assert_eq!(
            *releases.lock().unwrap(),
            vec![(62, vec![0x62, 0x63, 0x64])]
        );
        assert_eq!(*pool_frees.lock().unwrap(), vec![62]);
    }

    #[test]
    fn custom_buffer_pool_copy_on_write_detaches_without_recycling_copy() {
        #[derive(Debug)]
        struct PoolToken {
            id: usize,
            size: usize,
        }

        let releases = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let pool_frees = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));
        let release_capture = std::sync::Arc::clone(&releases);
        let pool_free_capture = std::sync::Arc::clone(&pool_frees);
        let pool = BufferPool::with_callbacks(
            3,
            0,
            BufferPoolCallbacks::with_allocation_callbacks(
                |allocated_len| {
                    assert_eq!(allocated_len, 3);
                    Ok(BufferPoolAllocation::with_opaque(
                        vec![1, 2, 3],
                        PoolToken {
                            id: 56,
                            size: allocated_len,
                        },
                    ))
                },
                move |allocation| {
                    let token = allocation
                        .opaque_ref::<PoolToken>()
                        .expect("pool token should be preserved");
                    release_capture
                        .lock()
                        .unwrap()
                        .push((token.id, allocation.as_slice().to_vec()));
                },
            )
            .with_pool_free(move || {
                pool_free_capture.lock().unwrap().push(56);
            }),
        )
        .unwrap();

        let source = pool.get().unwrap();
        let mut detached = BufferRef::ref_from(&source);
        detached.make_mut();

        assert_eq!(source.as_slice(), &[1, 2, 3]);
        assert_eq!(detached.as_slice(), &[1, 2, 3]);
        assert!(source.is_writable());
        assert!(detached.is_writable());
        assert!(!source.shares_storage(&detached));
        assert_eq!(
            source
                .pool_opaque_ref::<PoolToken>()
                .map(|token| (token.id, token.size)),
            Some((56, 3))
        );
        assert!(detached.pool_opaque_ref::<PoolToken>().is_none());

        detached.make_mut().copy_from_slice(&[0xab, 0xbc, 0xcd]);
        assert_eq!(detached.as_slice(), &[0xab, 0xbc, 0xcd]);
        assert_eq!(source.as_slice(), &[1, 2, 3]);

        drop(detached);
        assert!(releases.lock().unwrap().is_empty());
        assert!(pool_frees.lock().unwrap().is_empty());
        assert_eq!(pool.available_count().unwrap(), 0);

        drop(source);
        assert_eq!(pool.available_count().unwrap(), 1);
        assert!(releases.lock().unwrap().is_empty());

        let reused = pool.get().unwrap();
        assert_eq!(reused.as_slice(), &[1, 2, 3]);
        assert_eq!(
            reused
                .pool_opaque_ref::<PoolToken>()
                .map(|token| (token.id, token.size)),
            Some((56, 3))
        );
        drop(reused);
        drop(pool);

        assert_eq!(*releases.lock().unwrap(), vec![(56, vec![1, 2, 3])]);
        assert_eq!(*pool_frees.lock().unwrap(), vec![56]);
    }

    #[test]
    fn custom_buffer_pool_offset_allocation_reuses_original_base_storage() {
        #[derive(Debug)]
        struct PoolToken {
            id: usize,
        }

        let releases = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let release_capture = std::sync::Arc::clone(&releases);
        let pool = BufferPool::with_callbacks(
            3,
            0,
            BufferPoolCallbacks::with_allocation_callbacks(
                |allocated_len| {
                    assert_eq!(allocated_len, 3);
                    BufferPoolAllocation::with_opaque_visible_range(
                        vec![0xee, 0x31, 0x32, 0x33],
                        1,
                        3,
                        PoolToken { id: 88 },
                    )
                },
                move |allocation| {
                    let id = allocation
                        .opaque_ref::<PoolToken>()
                        .map(|token| token.id)
                        .unwrap_or_default();
                    release_capture
                        .lock()
                        .unwrap()
                        .push((id, allocation.into_vec()));
                },
            ),
        )
        .unwrap();

        let first = pool.get().unwrap();
        assert_eq!(first.offset(), 1);
        assert_eq!(first.len(), 3);
        assert_eq!(first.as_slice(), &[0x31, 0x32, 0x33]);
        assert_eq!(
            first.pool_opaque_ref::<PoolToken>().map(|token| token.id),
            Some(88)
        );
        drop(first);
        assert!(releases.lock().unwrap().is_empty());
        assert_eq!(pool.available_count().unwrap(), 1);

        let reuse = pool.get().unwrap();
        assert_eq!(reuse.offset(), 0);
        assert_eq!(reuse.len(), 3);
        assert_eq!(reuse.as_slice(), &[0xee, 0x31, 0x32]);
        assert_eq!(
            reuse.pool_opaque_ref::<PoolToken>().map(|token| token.id),
            Some(88)
        );
        drop(reuse);
        drop(pool);

        assert_eq!(
            *releases.lock().unwrap(),
            vec![(88, vec![0xee, 0x31, 0x32, 0x33])]
        );
    }

    #[test]
    fn custom_buffer_pool_offset_allocation_mutable_make_mut_preserves_visible_offset_shape() {
        #[derive(Debug)]
        struct PoolToken {
            id: usize,
        }

        let releases = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let release_capture = std::sync::Arc::clone(&releases);
        let pool = BufferPool::with_callbacks(
            3,
            0,
            BufferPoolCallbacks::with_allocation_callbacks(
                |allocated_len| {
                    assert_eq!(allocated_len, 3);
                    BufferPoolAllocation::with_opaque_visible_range(
                        vec![0xee, 0x31, 0x32, 0x33],
                        1,
                        3,
                        PoolToken { id: 90 },
                    )
                },
                move |allocation| {
                    let id = allocation
                        .opaque_ref::<PoolToken>()
                        .map(|token| token.id)
                        .unwrap_or_default();
                    release_capture
                        .lock()
                        .unwrap()
                        .push((id, allocation.into_vec()));
                },
            ),
        )
        .unwrap();

        let mut first = pool.get().unwrap();
        assert_eq!(first.offset(), 1);
        assert_eq!(first.len(), 3);
        assert_eq!(first.as_slice(), &[0x31, 0x32, 0x33]);
        first.make_mut()[0] = 0xaa;
        assert_eq!(first.offset(), 1);
        assert_eq!(first.as_slice(), &[0xaa, 0x32, 0x33]);
        drop(first);
        assert!(releases.lock().unwrap().is_empty());
        assert_eq!(pool.available_count().unwrap(), 1);

        let reuse = pool.get().unwrap();
        assert_eq!(reuse.offset(), 0);
        assert_eq!(reuse.len(), 3);
        assert_eq!(reuse.as_slice(), &[0xee, 0xaa, 0x32]);
        assert_eq!(
            reuse.pool_opaque_ref::<PoolToken>().map(|token| token.id),
            Some(90)
        );
        drop(reuse);
        drop(pool);
        assert_eq!(
            *releases.lock().unwrap(),
            vec![(90, vec![0xee, 0xaa, 0x32, 0x33])]
        );
    }

    #[test]
    fn custom_buffer_pool_readonly_offset_allocation_reuses_base_storage_as_writable() {
        #[derive(Debug)]
        struct PoolToken {
            id: usize,
        }

        let releases = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let release_capture = std::sync::Arc::clone(&releases);
        let pool = BufferPool::with_callbacks(
            3,
            0,
            BufferPoolCallbacks::with_allocation_callbacks(
                |allocated_len| {
                    assert_eq!(allocated_len, 3);
                    BufferPoolAllocation::with_opaque_readonly_visible_range(
                        vec![0xee, 0x31, 0x32, 0x33],
                        1,
                        3,
                        PoolToken { id: 89 },
                    )
                },
                move |allocation| {
                    let id = allocation
                        .opaque_ref::<PoolToken>()
                        .map(|token| token.id)
                        .unwrap_or_default();
                    release_capture
                        .lock()
                        .unwrap()
                        .push((id, allocation.into_vec()));
                },
            ),
        )
        .unwrap();

        let first = pool.get().unwrap();
        assert_eq!(first.offset(), 1);
        assert_eq!(first.len(), 3);
        assert_eq!(first.as_slice(), &[0x31, 0x32, 0x33]);
        assert!(!first.is_writable());
        assert_eq!(
            first.pool_opaque_ref::<PoolToken>().map(|token| token.id),
            Some(89)
        );
        drop(first);
        assert!(releases.lock().unwrap().is_empty());
        assert_eq!(pool.available_count().unwrap(), 1);

        let mut reuse = pool.get().unwrap();
        assert_eq!(reuse.offset(), 0);
        assert_eq!(reuse.len(), 3);
        assert_eq!(reuse.as_slice(), &[0xee, 0x31, 0x32]);
        assert!(reuse.is_writable());
        assert_eq!(
            reuse.pool_opaque_ref::<PoolToken>().map(|token| token.id),
            Some(89)
        );
        reuse.make_mut()[0] = 0xaa;
        drop(reuse);
        drop(pool);

        assert_eq!(
            *releases.lock().unwrap(),
            vec![(89, vec![0xaa, 0x31, 0x32, 0x33])]
        );
    }

    #[test]
    fn custom_buffer_pool_readonly_allocation_reuses_as_writable_storage() {
        #[derive(Debug)]
        struct PoolToken {
            id: usize,
        }

        let releases = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let pool_free_events = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));
        let release_capture = std::sync::Arc::clone(&releases);
        let pool_free_capture = std::sync::Arc::clone(&pool_free_events);
        let pool = BufferPool::with_callbacks(
            3,
            0,
            BufferPoolCallbacks::with_allocation_callbacks(
                |allocated_len| {
                    assert_eq!(allocated_len, 3);
                    Ok(BufferPoolAllocation::with_opaque_readonly(
                        vec![0x41, 0x42, 0x43],
                        PoolToken { id: 77 },
                    ))
                },
                move |allocation| {
                    let id = allocation
                        .opaque_ref::<PoolToken>()
                        .map(|token| token.id)
                        .unwrap_or_default();
                    release_capture
                        .lock()
                        .unwrap()
                        .push((id, allocation.into_vec()));
                },
            )
            .with_pool_free(move || {
                pool_free_capture.lock().unwrap().push(77);
            }),
        )
        .unwrap();

        let first = pool.get().unwrap();
        assert!(first.is_readonly());
        assert!(!first.is_writable());
        assert_eq!(first.as_slice(), &[0x41, 0x42, 0x43]);
        assert_eq!(
            first.pool_opaque_ref::<PoolToken>().map(|token| token.id),
            Some(77)
        );
        drop(first);
        assert!(releases.lock().unwrap().is_empty());
        assert!(pool_free_events.lock().unwrap().is_empty());

        let mut reused = pool.get().unwrap();
        assert!(!reused.is_readonly());
        assert!(reused.is_writable());
        assert_eq!(reused.as_slice(), &[0x41, 0x42, 0x43]);
        assert_eq!(
            reused.pool_opaque_ref::<PoolToken>().map(|token| token.id),
            Some(77)
        );
        reused.make_mut()[0] = 0xaa;
        drop(reused);
        drop(pool);

        assert_eq!(
            *releases.lock().unwrap(),
            vec![(77, vec![0xaa, 0x42, 0x43])]
        );
        assert_eq!(*pool_free_events.lock().unwrap(), vec![77]);
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

        assert_eq!(*releases.lock().unwrap(), vec![vec![7, 7, 7]]);
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
    fn custom_buffer_pool_allocator_failure_preserves_pool_until_drop() {
        let allocations = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));
        let releases = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
        let pool_frees = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));
        let allocate_capture = std::sync::Arc::clone(&allocations);
        let release_capture = std::sync::Arc::clone(&releases);
        let pool_free_capture = std::sync::Arc::clone(&pool_frees);
        let pool = BufferPool::with_callbacks(
            4,
            0,
            BufferPoolCallbacks::with_allocation_callbacks(
                move |allocated_len| {
                    allocate_capture.lock().unwrap().push(allocated_len);
                    Err(AvError::external("pool allocation failed"))
                },
                move |allocation| {
                    release_capture.lock().unwrap().push(allocation.into_vec());
                },
            )
            .with_pool_free(move || {
                pool_free_capture.lock().unwrap().push(77);
            }),
        )
        .unwrap();

        assert_eq!(pool.get().unwrap_err().kind(), AvErrorKind::External);
        assert_eq!(*allocations.lock().unwrap(), vec![4]);
        assert!(releases.lock().unwrap().is_empty());
        assert!(pool_frees.lock().unwrap().is_empty());
        assert_eq!(pool.available_count().unwrap(), 0);

        drop(pool);
        assert!(releases.lock().unwrap().is_empty());
        assert_eq!(*pool_frees.lock().unwrap(), vec![77]);
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
        assert_eq!(*pool_releases.lock().unwrap(), vec![vec![8, 9, 0]]);
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
        assert_eq!(reused.as_slice(), &[1, 2, 3, 4]);
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
    fn buffer_pool_uninit_with_shared_owned_refs_releases_once_on_final_drop() {
        let release_events = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
        let release_capture = std::sync::Arc::clone(&release_events);
        let release_lifecycle =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::<&'static str>::new()));
        let release_lifecycle_capture = std::sync::Arc::clone(&release_lifecycle);
        let pool_free_lifecycle_capture = std::sync::Arc::clone(&release_lifecycle);
        let pool_free_events =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::<&'static str>::new()));
        let pool_free_capture = std::sync::Arc::clone(&pool_free_events);

        let pool = BufferPool::with_callbacks(
            2,
            0,
            BufferPoolCallbacks::with_allocation_callbacks(
                |allocated_len| {
                    assert_eq!(allocated_len, 2);
                    Ok(BufferPoolAllocation::with_opaque(vec![1, 2], 77usize))
                },
                move |allocation| {
                    release_lifecycle_capture.lock().unwrap().push("release");
                    release_capture.lock().unwrap().push(allocation.into_vec());
                },
            )
            .with_pool_free(move || {
                pool_free_lifecycle_capture
                    .lock()
                    .unwrap()
                    .push("pool_free");
                pool_free_capture.lock().unwrap().push("pool_free");
            }),
        )
        .unwrap();

        let first = pool.get().unwrap();
        let second = first.clone();
        assert_eq!(first.strong_count(), 2);
        assert!(first.shares_storage(&second));

        let mut pool = Some(pool);
        BufferPool::uninit(&mut pool);
        assert!(pool.is_none());

        assert!(release_events.lock().unwrap().is_empty());
        assert!(pool_free_events.lock().unwrap().is_empty());

        drop(second);

        assert!(release_events.lock().unwrap().is_empty());
        assert!(pool_free_events.lock().unwrap().is_empty());

        drop(first);
        assert_eq!(
            *release_lifecycle.lock().unwrap(),
            vec!["release", "pool_free"]
        );
        assert_eq!(*release_events.lock().unwrap(), vec![vec![1, 2]]);
        assert_eq!(*pool_free_events.lock().unwrap(), vec!["pool_free"]);
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
        assert_eq!(reused.as_padded_slice(), &[8, 9, 0]);
    }

    #[test]
    fn buffer_realloc_detaches_pool_owned_storage_back_to_pool() {
        #[derive(Debug)]
        struct PoolToken {
            id: usize,
            size: usize,
        }

        let releases = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let pool_frees = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));
        let release_capture = std::sync::Arc::clone(&releases);
        let pool_free_capture = std::sync::Arc::clone(&pool_frees);
        let pool = BufferPool::with_callbacks(
            3,
            0,
            BufferPoolCallbacks::with_allocation_callbacks(
                |allocated_len| {
                    assert_eq!(allocated_len, 3);
                    Ok(BufferPoolAllocation::with_opaque(
                        vec![1, 2, 3],
                        PoolToken {
                            id: 57,
                            size: allocated_len,
                        },
                    ))
                },
                move |allocation| {
                    let token = allocation
                        .opaque_ref::<PoolToken>()
                        .expect("pool token should be preserved");
                    release_capture
                        .lock()
                        .unwrap()
                        .push((token.id, allocation.as_slice().to_vec()));
                },
            )
            .with_pool_free(move || {
                pool_free_capture.lock().unwrap().push(57);
            }),
        )
        .unwrap();

        let mut buffer = Some(pool.get().unwrap());
        buffer
            .as_mut()
            .unwrap()
            .make_mut()
            .copy_from_slice(&[0x10, 0x11, 0x12]);
        BufferRef::realloc(&mut buffer, 5).unwrap();
        let mut detached = buffer.expect("pool realloc keeps destination");

        assert_eq!(detached.len(), 5);
        assert_eq!(&detached.as_slice()[..3], &[0x10, 0x11, 0x12]);
        assert!(detached.as_slice()[3..].iter().all(|byte| *byte == 0));
        assert!(detached.is_writable());
        assert!(detached.pool_opaque_ref::<PoolToken>().is_none());
        assert!(releases.lock().unwrap().is_empty());
        assert!(pool_frees.lock().unwrap().is_empty());
        assert_eq!(pool.available_count().unwrap(), 1);

        let reused = pool.get().unwrap();
        assert_eq!(reused.as_slice(), &[0x10, 0x11, 0x12]);
        assert_eq!(
            reused
                .pool_opaque_ref::<PoolToken>()
                .map(|token| (token.id, token.size)),
            Some((57, 3))
        );

        detached.make_mut()[0] = 0xee;
        assert_eq!(&detached.as_slice()[..3], &[0xee, 0x11, 0x12]);
        assert_eq!(reused.as_slice(), &[0x10, 0x11, 0x12]);
        drop(detached);
        assert!(releases.lock().unwrap().is_empty());

        drop(reused);
        drop(pool);
        assert_eq!(
            *releases.lock().unwrap(),
            vec![(57, vec![0x10, 0x11, 0x12])]
        );
        assert_eq!(*pool_frees.lock().unwrap(), vec![57]);
    }

    #[test]
    fn buffer_replace_returns_pool_owned_destination_storage_to_pool() {
        #[derive(Debug)]
        struct PoolToken {
            id: usize,
            size: usize,
        }

        let releases = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let pool_frees = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));
        let release_capture = std::sync::Arc::clone(&releases);
        let pool_free_capture = std::sync::Arc::clone(&pool_frees);
        let pool = BufferPool::with_callbacks(
            3,
            0,
            BufferPoolCallbacks::with_allocation_callbacks(
                |allocated_len| {
                    assert_eq!(allocated_len, 3);
                    Ok(BufferPoolAllocation::with_opaque(
                        vec![1, 2, 3],
                        PoolToken {
                            id: 58,
                            size: allocated_len,
                        },
                    ))
                },
                move |allocation| {
                    let token = allocation
                        .opaque_ref::<PoolToken>()
                        .expect("pool token should be preserved");
                    release_capture
                        .lock()
                        .unwrap()
                        .push((token.id, allocation.as_slice().to_vec()));
                },
            )
            .with_pool_free(move || {
                pool_free_capture.lock().unwrap().push(58);
            }),
        )
        .unwrap();

        let source = BufferRef::from_vec(vec![0x91, 0x92]);
        let mut destination = Some(pool.get().unwrap());
        destination
            .as_mut()
            .unwrap()
            .make_mut()
            .copy_from_slice(&[0x20, 0x21, 0x22]);
        BufferRef::replace(&mut destination, Some(&source));
        let replaced = destination.expect("replace keeps destination");

        assert!(replaced.shares_storage(&source));
        assert_eq!(replaced.as_slice(), &[0x91, 0x92]);
        assert!(replaced.pool_opaque_ref::<PoolToken>().is_none());
        assert!(releases.lock().unwrap().is_empty());
        assert!(pool_frees.lock().unwrap().is_empty());
        assert_eq!(pool.available_count().unwrap(), 1);

        let reused = pool.get().unwrap();
        assert_eq!(reused.as_slice(), &[0x20, 0x21, 0x22]);
        assert_eq!(
            reused
                .pool_opaque_ref::<PoolToken>()
                .map(|token| (token.id, token.size)),
            Some((58, 3))
        );

        drop(replaced);
        assert!(releases.lock().unwrap().is_empty());
        drop(reused);
        drop(pool);
        assert_eq!(
            *releases.lock().unwrap(),
            vec![(58, vec![0x20, 0x21, 0x22])]
        );
        assert_eq!(*pool_frees.lock().unwrap(), vec![58]);
    }

    #[test]
    fn buffer_replace_null_returns_pool_owned_destination_storage_to_pool() {
        #[derive(Debug)]
        struct PoolToken {
            id: usize,
            size: usize,
        }

        let releases = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let pool_frees = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));
        let release_capture = std::sync::Arc::clone(&releases);
        let pool_free_capture = std::sync::Arc::clone(&pool_frees);
        let pool = BufferPool::with_callbacks(
            3,
            0,
            BufferPoolCallbacks::with_allocation_callbacks(
                |allocated_len| {
                    assert_eq!(allocated_len, 3);
                    Ok(BufferPoolAllocation::with_opaque(
                        vec![1, 2, 3],
                        PoolToken {
                            id: 63,
                            size: allocated_len,
                        },
                    ))
                },
                move |allocation| {
                    let token = allocation
                        .opaque_ref::<PoolToken>()
                        .expect("pool token should be preserved");
                    release_capture
                        .lock()
                        .unwrap()
                        .push((token.id, allocation.as_slice().to_vec()));
                },
            )
            .with_pool_free(move || {
                pool_free_capture.lock().unwrap().push(63);
            }),
        )
        .unwrap();

        let mut destination = Some(pool.get().unwrap());
        destination
            .as_mut()
            .unwrap()
            .make_mut()
            .copy_from_slice(&[0x63, 0x64, 0x65]);
        BufferRef::replace(&mut destination, None);

        assert!(destination.is_none());
        assert!(releases.lock().unwrap().is_empty());
        assert!(pool_frees.lock().unwrap().is_empty());
        assert_eq!(pool.available_count().unwrap(), 1);

        let reused = pool.get().unwrap();
        assert_eq!(reused.as_slice(), &[0x63, 0x64, 0x65]);
        assert_eq!(
            reused
                .pool_opaque_ref::<PoolToken>()
                .map(|token| (token.id, token.size)),
            Some((63, 3))
        );

        drop(reused);
        drop(pool);
        assert_eq!(
            *releases.lock().unwrap(),
            vec![(63, vec![0x63, 0x64, 0x65])]
        );
        assert_eq!(*pool_frees.lock().unwrap(), vec![63]);
    }

    #[test]
    fn buffer_replace_shares_pool_owned_source_until_destination_unref() {
        #[derive(Debug)]
        struct PoolToken {
            id: usize,
            size: usize,
        }

        let releases = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let pool_frees = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));
        let release_capture = std::sync::Arc::clone(&releases);
        let pool_free_capture = std::sync::Arc::clone(&pool_frees);
        let pool = BufferPool::with_callbacks(
            3,
            0,
            BufferPoolCallbacks::with_allocation_callbacks(
                |allocated_len| {
                    assert_eq!(allocated_len, 3);
                    Ok(BufferPoolAllocation::with_opaque(
                        vec![1, 2, 3],
                        PoolToken {
                            id: 59,
                            size: allocated_len,
                        },
                    ))
                },
                move |allocation| {
                    let token = allocation
                        .opaque_ref::<PoolToken>()
                        .expect("pool token should be preserved");
                    release_capture
                        .lock()
                        .unwrap()
                        .push((token.id, allocation.as_slice().to_vec()));
                },
            )
            .with_pool_free(move || {
                pool_free_capture.lock().unwrap().push(59);
            }),
        )
        .unwrap();

        let mut source = pool.get().unwrap();
        source.make_mut().copy_from_slice(&[0x41, 0x42, 0x43]);
        let mut destination = Some(BufferRef::from_vec(vec![0x66, 0x67]));
        BufferRef::replace(&mut destination, Some(&source));
        let replaced = destination.expect("replace keeps destination");

        assert!(replaced.shares_storage(&source));
        assert_eq!(replaced.as_slice(), &[0x41, 0x42, 0x43]);
        assert_eq!(
            replaced
                .pool_opaque_ref::<PoolToken>()
                .map(|token| (token.id, token.size)),
            Some((59, 3))
        );
        assert!(releases.lock().unwrap().is_empty());
        assert!(pool_frees.lock().unwrap().is_empty());
        assert_eq!(pool.available_count().unwrap(), 0);

        drop(source);
        assert!(releases.lock().unwrap().is_empty());
        assert_eq!(pool.available_count().unwrap(), 0);

        drop(replaced);
        assert!(releases.lock().unwrap().is_empty());
        assert_eq!(pool.available_count().unwrap(), 1);

        let reused = pool.get().unwrap();
        assert_eq!(reused.as_slice(), &[0x41, 0x42, 0x43]);
        assert_eq!(
            reused
                .pool_opaque_ref::<PoolToken>()
                .map(|token| (token.id, token.size)),
            Some((59, 3))
        );

        drop(reused);
        drop(pool);
        assert_eq!(
            *releases.lock().unwrap(),
            vec![(59, vec![0x41, 0x42, 0x43])]
        );
        assert_eq!(*pool_frees.lock().unwrap(), vec![59]);
    }

    #[test]
    fn buffer_replace_between_distinct_pools_preserves_both_pool_lifecycles() {
        #[derive(Debug)]
        struct PoolToken {
            id: usize,
            size: usize,
        }

        let destination_releases =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let destination_pool_frees =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));
        let destination_release_capture = std::sync::Arc::clone(&destination_releases);
        let destination_pool_free_capture = std::sync::Arc::clone(&destination_pool_frees);
        let destination_pool = BufferPool::with_callbacks(
            3,
            0,
            BufferPoolCallbacks::with_allocation_callbacks(
                |allocated_len| {
                    assert_eq!(allocated_len, 3);
                    Ok(BufferPoolAllocation::with_opaque(
                        vec![1, 2, 3],
                        PoolToken {
                            id: 60,
                            size: allocated_len,
                        },
                    ))
                },
                move |allocation| {
                    let token = allocation
                        .opaque_ref::<PoolToken>()
                        .expect("destination pool token should be preserved");
                    destination_release_capture
                        .lock()
                        .unwrap()
                        .push((token.id, allocation.as_slice().to_vec()));
                },
            )
            .with_pool_free(move || {
                destination_pool_free_capture.lock().unwrap().push(60);
            }),
        )
        .unwrap();

        let source_releases =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let source_pool_frees = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));
        let source_release_capture = std::sync::Arc::clone(&source_releases);
        let source_pool_free_capture = std::sync::Arc::clone(&source_pool_frees);
        let source_pool = BufferPool::with_callbacks(
            3,
            0,
            BufferPoolCallbacks::with_allocation_callbacks(
                |allocated_len| {
                    assert_eq!(allocated_len, 3);
                    Ok(BufferPoolAllocation::with_opaque(
                        vec![4, 5, 6],
                        PoolToken {
                            id: 61,
                            size: allocated_len,
                        },
                    ))
                },
                move |allocation| {
                    let token = allocation
                        .opaque_ref::<PoolToken>()
                        .expect("source pool token should be preserved");
                    source_release_capture
                        .lock()
                        .unwrap()
                        .push((token.id, allocation.as_slice().to_vec()));
                },
            )
            .with_pool_free(move || {
                source_pool_free_capture.lock().unwrap().push(61);
            }),
        )
        .unwrap();

        let mut source = source_pool.get().unwrap();
        source.make_mut().copy_from_slice(&[0x51, 0x52, 0x53]);
        let mut destination = Some(destination_pool.get().unwrap());
        destination
            .as_mut()
            .unwrap()
            .make_mut()
            .copy_from_slice(&[0x31, 0x32, 0x33]);

        BufferRef::replace(&mut destination, Some(&source));
        let replaced = destination.expect("replace keeps destination");

        assert!(replaced.shares_storage(&source));
        assert_eq!(replaced.as_slice(), &[0x51, 0x52, 0x53]);
        assert_eq!(
            replaced
                .pool_opaque_ref::<PoolToken>()
                .map(|token| (token.id, token.size)),
            Some((61, 3))
        );
        assert!(destination_releases.lock().unwrap().is_empty());
        assert!(source_releases.lock().unwrap().is_empty());
        assert!(destination_pool_frees.lock().unwrap().is_empty());
        assert!(source_pool_frees.lock().unwrap().is_empty());
        assert_eq!(destination_pool.available_count().unwrap(), 1);
        assert_eq!(source_pool.available_count().unwrap(), 0);

        let destination_reuse = destination_pool.get().unwrap();
        assert_eq!(destination_reuse.as_slice(), &[0x31, 0x32, 0x33]);
        assert_eq!(
            destination_reuse
                .pool_opaque_ref::<PoolToken>()
                .map(|token| (token.id, token.size)),
            Some((60, 3))
        );

        drop(source);
        assert!(source_releases.lock().unwrap().is_empty());
        assert_eq!(source_pool.available_count().unwrap(), 0);

        drop(replaced);
        assert!(source_releases.lock().unwrap().is_empty());
        assert_eq!(source_pool.available_count().unwrap(), 1);

        let source_reuse = source_pool.get().unwrap();
        assert_eq!(source_reuse.as_slice(), &[0x51, 0x52, 0x53]);
        assert_eq!(
            source_reuse
                .pool_opaque_ref::<PoolToken>()
                .map(|token| (token.id, token.size)),
            Some((61, 3))
        );

        drop(destination_reuse);
        drop(destination_pool);
        assert_eq!(
            *destination_releases.lock().unwrap(),
            vec![(60, vec![0x31, 0x32, 0x33])]
        );
        assert_eq!(*destination_pool_frees.lock().unwrap(), vec![60]);
        assert!(source_releases.lock().unwrap().is_empty());
        assert!(source_pool_frees.lock().unwrap().is_empty());

        drop(source_reuse);
        drop(source_pool);
        assert_eq!(
            *source_releases.lock().unwrap(),
            vec![(61, vec![0x51, 0x52, 0x53])]
        );
        assert_eq!(*source_pool_frees.lock().unwrap(), vec![61]);
    }

    #[test]
    fn buffer_replace_with_same_pool_reuses_destination_spare_and_lifo_release() {
        #[derive(Debug)]
        struct PoolToken {
            id: usize,
            size: usize,
        }

        let releases = std::sync::Arc::new(std::sync::Mutex::new(Vec::<(usize, Vec<u8>)>::new()));
        let pool_frees = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));
        let release_capture = std::sync::Arc::clone(&releases);
        let pool_free_capture = std::sync::Arc::clone(&pool_frees);
        let pool = BufferPool::with_callbacks(
            3,
            0,
            BufferPoolCallbacks::with_allocation_callbacks(
                |allocated_len| {
                    assert_eq!(allocated_len, 3);
                    Ok(BufferPoolAllocation::with_opaque(
                        vec![1, 2, 3],
                        PoolToken {
                            id: 64,
                            size: allocated_len,
                        },
                    ))
                },
                move |allocation| {
                    let token = allocation
                        .opaque_ref::<PoolToken>()
                        .expect("pool token should be preserved");
                    release_capture
                        .lock()
                        .unwrap()
                        .push((token.id, allocation.as_slice().to_vec()));
                },
            )
            .with_pool_free(move || {
                pool_free_capture.lock().unwrap().push(64);
            }),
        )
        .unwrap();

        let mut source = pool.get().unwrap();
        source.make_mut().copy_from_slice(&[0x51, 0x52, 0x53]);
        let mut destination = Some(pool.get().unwrap());
        destination
            .as_mut()
            .unwrap()
            .make_mut()
            .copy_from_slice(&[0x31, 0x32, 0x33]);

        BufferRef::replace(&mut destination, Some(&source));
        let replaced = destination.expect("replace keeps destination");

        assert!(replaced.shares_storage(&source));
        assert_eq!(replaced.strong_count(), 2);
        assert_eq!(replaced.as_slice(), &[0x51, 0x52, 0x53]);
        assert_eq!(
            replaced
                .pool_opaque_ref::<PoolToken>()
                .map(|token| (token.id, token.size)),
            Some((64, 3))
        );
        assert!(releases.lock().unwrap().is_empty());
        assert!(pool_frees.lock().unwrap().is_empty());
        assert_eq!(pool.available_count().unwrap(), 1);

        let destination_reuse = pool.get().unwrap();
        assert_eq!(destination_reuse.as_slice(), &[0x31, 0x32, 0x33]);
        assert_eq!(
            destination_reuse
                .pool_opaque_ref::<PoolToken>()
                .map(|token| (token.id, token.size)),
            Some((64, 3))
        );
        assert!(releases.lock().unwrap().is_empty());
        assert!(pool_frees.lock().unwrap().is_empty());
        assert_eq!(pool.available_count().unwrap(), 0);

        drop(destination_reuse);
        assert!(releases.lock().unwrap().is_empty());
        assert!(pool_frees.lock().unwrap().is_empty());
        assert_eq!(pool.available_count().unwrap(), 1);

        drop(source);
        assert!(releases.lock().unwrap().is_empty());
        assert_eq!(pool.available_count().unwrap(), 1);

        drop(replaced);
        assert!(releases.lock().unwrap().is_empty());
        assert_eq!(pool.available_count().unwrap(), 2);

        let source_reuse = pool.get().unwrap();
        let destination_reuse_again = pool.get().unwrap();
        assert_eq!(source_reuse.as_slice(), &[0x51, 0x52, 0x53]);
        assert_eq!(destination_reuse_again.as_slice(), &[0x31, 0x32, 0x33]);
        assert_eq!(
            source_reuse
                .pool_opaque_ref::<PoolToken>()
                .map(|token| (token.id, token.size)),
            Some((64, 3))
        );
        assert_eq!(
            destination_reuse_again
                .pool_opaque_ref::<PoolToken>()
                .map(|token| (token.id, token.size)),
            Some((64, 3))
        );
        assert_eq!(pool.available_count().unwrap(), 0);

        drop(destination_reuse_again);
        drop(source_reuse);
        drop(pool);
        assert_eq!(
            *releases.lock().unwrap(),
            vec![(64, vec![0x51, 0x52, 0x53]), (64, vec![0x31, 0x32, 0x33])]
        );
        assert_eq!(*pool_frees.lock().unwrap(), vec![64]);
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
    fn buffer_pool_recycle_rejections_release_or_retain_by_ownership() {
        let caller_releases = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
        let pool_releases = std::sync::Arc::new(std::sync::Mutex::new(Vec::<Vec<u8>>::new()));
        let pool_release_capture = std::sync::Arc::clone(&pool_releases);
        let pool = BufferPool::with_callbacks(
            2,
            1,
            BufferPoolCallbacks::new(
                |allocated_len| Ok(vec![0; allocated_len]),
                move |storage| {
                    pool_release_capture.lock().unwrap().push(storage);
                },
            ),
        )
        .unwrap();

        let wrong_offset_capture = std::sync::Arc::clone(&caller_releases);
        let wrong_offset = BufferRef::from_vec_with_len_and_release_callback(
            vec![0xee, 0x11, 0x22, 0],
            3,
            move |storage| {
                wrong_offset_capture.lock().unwrap().push(storage);
            },
        )
        .unwrap()
        .into_ref_slice(1, 2)
        .unwrap();
        assert_eq!(
            pool.recycle(wrong_offset).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );

        let wrong_len_capture = std::sync::Arc::clone(&caller_releases);
        let wrong_len = BufferRef::from_vec_with_len_and_release_callback(
            vec![0x31, 0x32, 0x33, 0],
            3,
            move |storage| {
                wrong_len_capture.lock().unwrap().push(storage);
            },
        )
        .unwrap();
        assert_eq!(
            pool.recycle(wrong_len).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );

        let wrong_padding_capture = std::sync::Arc::clone(&caller_releases);
        let wrong_padding = BufferRef::from_vec_with_len_and_release_callback(
            vec![0x41, 0x42, 0, 0],
            2,
            move |storage| {
                wrong_padding_capture.lock().unwrap().push(storage);
            },
        )
        .unwrap();
        assert_eq!(
            pool.recycle(wrong_padding).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );

        let readonly_capture = std::sync::Arc::clone(&caller_releases);
        let readonly = BufferRef::from_vec_with_len_and_release_callback_readonly(
            vec![0x51, 0x52, 0],
            2,
            move |storage| {
                readonly_capture.lock().unwrap().push(storage);
            },
        )
        .unwrap();
        assert_eq!(
            pool.recycle(readonly).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );

        assert_eq!(
            *caller_releases.lock().unwrap(),
            vec![
                vec![0xee, 0x11, 0x22, 0],
                vec![0x31, 0x32, 0x33, 0],
                vec![0x41, 0x42, 0, 0],
                vec![0x51, 0x52, 0],
            ]
        );
        assert!(pool_releases.lock().unwrap().is_empty());
        assert_eq!(pool.available_count().unwrap(), 0);

        let mut shared = pool.get().unwrap();
        shared.make_mut().copy_from_slice(&[0xa1, 0xa2]);
        let shared_survivor = shared.clone();
        assert_eq!(
            pool.recycle(shared).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
        assert_eq!(shared_survivor.as_slice(), &[0xa1, 0xa2]);
        assert_eq!(pool.available_count().unwrap(), 0);
        assert!(pool_releases.lock().unwrap().is_empty());

        drop(shared_survivor);
        assert_eq!(pool.available_count().unwrap(), 1);
        drop(pool);
        assert_eq!(*pool_releases.lock().unwrap(), vec![vec![0xa1, 0xa2, 0]]);
    }

    #[test]
    fn buffer_pool_rejects_overflowing_shapes() {
        assert_eq!(
            BufferPool::new(1, usize::MAX).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );

        let pool_frees = std::sync::Arc::new(std::sync::Mutex::new(Vec::<usize>::new()));
        let pool_free_capture = std::sync::Arc::clone(&pool_frees);
        let huge_pool = BufferPool::with_callbacks(
            usize::MAX,
            0,
            BufferPoolCallbacks::default().with_pool_free(move || {
                pool_free_capture.lock().unwrap().push(99);
            }),
        )
        .unwrap();
        let get_err = huge_pool.get().unwrap_err();
        assert_eq!(get_err.kind(), AvErrorKind::External);
        assert_eq!(get_err.code(), Some(AvErrorCode::ENOMEM));
        assert!(pool_frees.lock().unwrap().is_empty());
        assert_eq!(huge_pool.available_count().unwrap(), 0);
        drop(huge_pool);
        assert_eq!(*pool_frees.lock().unwrap(), vec![99]);
    }
}
