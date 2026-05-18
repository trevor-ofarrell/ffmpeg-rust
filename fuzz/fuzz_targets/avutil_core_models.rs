#![no_main]

use avutil::{
    adler32, crc32_ieee, digest_to_hex, md5, rescale_q, rescale_q_rnd, rescale_q_rnd_pass_minmax,
    sha224, sha256, sha384, sha512, Adler32, AudioFrame, AvError, AvErrorKind, BufferPool,
    BufferPoolCallbacks, BufferRef, Channel, ChannelLayout, Crc32, Frame,
    FrameActiveFormatDescription, FrameAudioServiceType, FrameData, FrameDisplayMatrix,
    FrameDownmixInfo, FrameDownmixType, FrameGopTimecode, FrameMatrixEncoding, FrameS12mTimecode,
    FrameSeiUnregistered, FrameSideData, FrameSideDataKind, FrameSideDataProperties,
    FrameSkipSamples, FrameSkipSamplesReason, Md5, Packet, PacketFlags, PixelFormat, Rational,
    Rounding, SampleFormat, SampleFormatNumericKind, Sha224, Sha256, Sha384, Sha512, SideData,
    VideoFrame,
};
use libfuzzer_sys::fuzz_target;
use std::io;
use std::sync::{Arc, Mutex};

const MAX_PAYLOAD: usize = 128;
const MAX_SAMPLES: usize = 64;

fuzz_target!(|data: &[u8]| {
    let mut cursor = Cursor::new(data);

    exercise_errors(&mut cursor);
    exercise_buffers(&mut cursor);
    exercise_rational_and_timebase(&mut cursor);
    exercise_pixel_and_video_frame(&mut cursor);
    exercise_sample_channel_and_audio_frame(&mut cursor);
    exercise_packet_and_hashes(&mut cursor);
    exercise_fixtures();
});

fn exercise_buffers(cursor: &mut Cursor<'_>) {
    let payload_len = usize::from(cursor.next().unwrap_or_default()) % (MAX_PAYLOAD + 1);
    let payload = payload_from(cursor, payload_len);
    let mut buffer = if cursor.next().unwrap_or_default().is_multiple_of(2) {
        BufferRef::from_vec(payload.clone())
    } else {
        BufferRef::copy_from_slice(&payload)
    };

    assert_eq!(buffer.as_slice(), payload.as_slice());
    assert_eq!(buffer.len(), payload.len());
    assert_eq!(buffer.is_empty(), payload.is_empty());
    assert!(buffer.is_writable());

    let offset = if buffer.is_empty() {
        0
    } else {
        usize::from(cursor.next().unwrap_or_default()) % (buffer.len() + 1)
    };
    let remaining = buffer.len() - offset;
    let slice_len = if remaining == 0 {
        0
    } else {
        usize::from(cursor.next().unwrap_or_default()) % (remaining + 1)
    };
    let slice = buffer.slice(offset, slice_len).unwrap();
    assert_eq!(slice.offset(), offset);
    assert_eq!(slice.len(), slice_len);
    assert_eq!(slice.as_slice(), &payload[offset..offset + slice_len]);
    assert!(buffer.shares_storage_with_slice(&slice));
    assert!(slice.shares_storage_with_buffer(&buffer));
    assert_eq!(buffer.as_ptr(), buffer.as_slice().as_ptr());
    assert_eq!(buffer.as_padded_ptr(), buffer.as_padded_slice().as_ptr());
    assert_eq!(slice.as_ptr(), buffer.as_ptr().wrapping_add(offset));
    assert_eq!(buffer.strong_count(), 2);
    assert_eq!(slice.strong_count(), 2);
    assert_eq!(
        buffer
            .slice(buffer.len().saturating_add(1), 0)
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidArgument
    );
    drop(slice);

    let shared = buffer.clone();
    assert!(buffer.shares_storage(&shared));
    assert!(!buffer.is_writable());
    assert!(buffer.get_mut().is_none());
    if !buffer.is_empty() {
        let original = buffer.as_slice()[0];
        buffer.make_mut()[0] = original.wrapping_add(1);
        assert_eq!(buffer.as_slice()[0], original.wrapping_add(1));
        assert_eq!(shared.as_slice(), payload.as_slice());
        assert!(!buffer.shares_storage(&shared));
    } else {
        assert_eq!(buffer.make_mut(), &mut []);
        assert_eq!(shared.as_slice(), payload.as_slice());
        assert!(!buffer.shares_storage(&shared));
    }
    assert!(buffer.is_writable());

    let zeroed_len = usize::from(cursor.next().unwrap_or_default()) % (MAX_PAYLOAD + 1);
    let zeroed = BufferRef::zeroed(zeroed_len).unwrap();
    assert_eq!(zeroed.len(), zeroed_len);
    assert_eq!(zeroed.allocated_len(), zeroed_len);
    assert_eq!(zeroed.padding_len(), 0);
    assert!(zeroed.as_slice().iter().all(|byte| *byte == 0));

    let padding_len = usize::from(cursor.next().unwrap_or_default()) % (MAX_PAYLOAD + 1);
    let mut padded = BufferRef::copy_from_slice_with_padding(&payload, padding_len).unwrap();
    assert_eq!(padded.len(), payload.len());
    assert_eq!(padded.allocated_len(), payload.len() + padding_len);
    assert_eq!(padded.padding_len(), padding_len);
    assert_eq!(padded.as_slice(), payload.as_slice());
    assert_eq!(
        &padded.as_padded_slice()[..payload.len()],
        payload.as_slice()
    );
    assert!(padded.padding_slice().iter().all(|byte| *byte == 0));
    assert_eq!(padded.slice(padded.len(), 0).unwrap().as_slice(), &[]);
    assert_eq!(
        padded
            .slice(padded.len().saturating_add(1), 0)
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidArgument
    );
    if !padded.is_empty() {
        let shared = padded.clone();
        let original = padded.as_slice()[0];
        padded.make_mut()[0] = original.wrapping_sub(1);
        assert_eq!(padded.as_slice()[0], original.wrapping_sub(1));
        assert!(padded.padding_slice().iter().all(|byte| *byte == 0));
        assert_eq!(shared.as_slice(), payload.as_slice());
        assert!(shared.padding_slice().iter().all(|byte| *byte == 0));
    }

    let padded_zeroed = BufferRef::zeroed_with_padding(payload_len, padding_len).unwrap();
    assert_eq!(padded_zeroed.len(), payload_len);
    assert_eq!(padded_zeroed.allocated_len(), payload_len + padding_len);
    assert!(padded_zeroed
        .as_padded_slice()
        .iter()
        .all(|byte| *byte == 0));
    assert_eq!(
        BufferRef::zeroed_with_padding(1, usize::MAX)
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidArgument
    );

    let resize_len = usize::from(cursor.next().unwrap_or_default()) % (MAX_PAYLOAD + 1);
    let resize_padding = usize::from(cursor.next().unwrap_or_default()) % (MAX_PAYLOAD + 1);
    let mut resizable = BufferRef::copy_from_slice_with_padding(&payload, padding_len).unwrap();
    resizable
        .resize_with_padding(resize_len, resize_padding)
        .unwrap();
    assert_eq!(resizable.len(), resize_len);
    assert_eq!(resizable.allocated_len(), resize_len + resize_padding);
    let copied = payload.len().min(resize_len);
    assert_eq!(&resizable.as_slice()[..copied], &payload[..copied]);
    assert!(resizable.as_slice()[copied..].iter().all(|byte| *byte == 0));
    assert!(resizable.padding_slice().iter().all(|byte| *byte == 0));
    let before_failed_resize = resizable.as_padded_slice().to_vec();
    let before_failed_len = resizable.len();
    assert_eq!(
        resizable
            .resize_with_padding(1, usize::MAX)
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidArgument
    );
    assert_eq!(resizable.as_padded_slice(), before_failed_resize.as_slice());
    assert_eq!(resizable.len(), before_failed_len);

    let mut resize_shared = BufferRef::copy_from_slice(&payload);
    let resize_original = resize_shared.clone();
    assert!(resize_shared.shares_storage(&resize_original));
    resize_shared
        .resize_with_padding(resize_len, resize_padding)
        .unwrap();
    assert!(resize_shared.is_writable());
    assert_eq!(resize_original.as_slice(), payload.as_slice());
    if resize_len != payload.len() || resize_padding != 0 {
        assert!(!resize_shared.shares_storage(&resize_original));
    }

    let mut same_shape_storage = payload.clone();
    same_shape_storage.resize(payload.len() + padding_len, 0x55);
    let mut same_shape =
        BufferRef::from_vec_with_len_and_release_callback(same_shape_storage, payload.len(), drop)
            .unwrap();
    same_shape
        .resize_with_padding(payload.len(), padding_len)
        .unwrap();
    assert_eq!(same_shape.as_slice(), payload.as_slice());
    assert!(same_shape.padding_slice().iter().all(|byte| *byte == 0));

    static STATIC_BUFFER_BYTES: &[u8] = b"static-buffer-fixture";
    let static_visible_len =
        usize::from(cursor.next().unwrap_or_default()) % (STATIC_BUFFER_BYTES.len() + 1);
    let mut static_buffer =
        BufferRef::from_static_slice_with_len_readonly(STATIC_BUFFER_BYTES, static_visible_len)
            .unwrap();
    assert_eq!(
        static_buffer.as_slice(),
        &STATIC_BUFFER_BYTES[..static_visible_len]
    );
    assert!(std::ptr::eq(
        static_buffer.as_padded_slice().as_ptr(),
        STATIC_BUFFER_BYTES.as_ptr()
    ));
    assert!(static_buffer.is_readonly());
    assert!(!static_buffer.is_writable());
    assert!(static_buffer.get_mut().is_none());
    let static_shared = static_buffer.clone();
    static_buffer
        .resize_with_padding(resize_len, resize_padding)
        .unwrap();
    assert!(static_buffer.is_writable());
    assert!(!static_buffer.is_readonly());
    let static_copied = static_visible_len.min(resize_len);
    assert_eq!(
        &static_buffer.as_slice()[..static_copied],
        &STATIC_BUFFER_BYTES[..static_copied]
    );
    assert!(static_buffer.as_slice()[static_copied..]
        .iter()
        .all(|byte| *byte == 0));
    assert!(static_buffer.padding_slice().iter().all(|byte| *byte == 0));
    assert_eq!(
        static_shared.as_slice(),
        &STATIC_BUFFER_BYTES[..static_visible_len]
    );
    assert!(static_shared.is_readonly());
    assert!(std::ptr::eq(
        static_shared.as_padded_slice().as_ptr(),
        STATIC_BUFFER_BYTES.as_ptr()
    ));

    let shared_storage: Arc<[u8]> = payload.clone().into();
    let mut shared_readonly = BufferRef::from_shared_slice_readonly(Arc::clone(&shared_storage));
    assert_eq!(shared_readonly.as_slice(), payload.as_slice());
    assert!(std::ptr::eq(
        shared_readonly.as_padded_slice().as_ptr(),
        shared_storage.as_ptr()
    ));
    assert!(shared_readonly.is_readonly());
    assert!(!shared_readonly.is_writable());
    assert!(shared_readonly.get_mut().is_none());
    assert_eq!(Arc::strong_count(&shared_storage), 2);
    let shared_readonly_original = shared_readonly.clone();
    shared_readonly
        .resize_with_padding(resize_len, resize_padding)
        .unwrap();
    assert!(shared_readonly.is_writable());
    assert!(!shared_readonly.is_readonly());
    let shared_copied = payload.len().min(resize_len);
    assert_eq!(
        &shared_readonly.as_slice()[..shared_copied],
        &payload[..shared_copied]
    );
    assert!(shared_readonly.as_slice()[shared_copied..]
        .iter()
        .all(|byte| *byte == 0));
    assert!(shared_readonly
        .padding_slice()
        .iter()
        .all(|byte| *byte == 0));
    assert_eq!(shared_readonly_original.as_slice(), payload.as_slice());
    assert!(std::ptr::eq(
        shared_readonly_original.as_padded_slice().as_ptr(),
        shared_storage.as_ptr()
    ));
    drop(shared_readonly_original);
    assert_eq!(Arc::strong_count(&shared_storage), 1);
    assert_eq!(
        BufferRef::from_shared_slice_with_len_readonly(
            Arc::clone(&shared_storage),
            shared_storage.len().saturating_add(1)
        )
        .unwrap_err()
        .kind(),
        AvErrorKind::InvalidArgument
    );
    assert_eq!(Arc::strong_count(&shared_storage), 1);

    let external_storage: Arc<[u8]> = payload.clone().into();
    let external_released = Arc::new(Mutex::new(Vec::<usize>::new()));
    let external_capture = Arc::clone(&external_released);
    let mut external_readonly = BufferRef::from_external_slice_with_len_and_opaque_readonly(
        Arc::clone(&external_storage),
        payload.len(),
        payload_len,
        move |opaque| {
            external_capture.lock().unwrap().push(opaque);
        },
    )
    .unwrap();
    assert_eq!(external_readonly.as_slice(), payload.as_slice());
    assert!(std::ptr::eq(
        external_readonly.as_padded_slice().as_ptr(),
        external_storage.as_ptr()
    ));
    assert!(external_readonly.is_readonly());
    assert!(!external_readonly.is_writable());
    assert_eq!(external_readonly.opaque_ref::<usize>(), Some(&payload_len));
    assert!(external_readonly.opaque_ref::<u8>().is_none());
    let external_original = external_readonly.clone();
    assert_eq!(external_original.opaque_ref::<usize>(), Some(&payload_len));
    external_readonly
        .resize_with_padding(resize_len, resize_padding)
        .unwrap();
    assert!(external_released.lock().unwrap().is_empty());
    assert!(external_readonly.is_writable());
    assert!(!external_readonly.is_readonly());
    assert!(external_readonly.opaque_ref::<usize>().is_none());
    let external_copied = payload.len().min(resize_len);
    assert_eq!(
        &external_readonly.as_slice()[..external_copied],
        &payload[..external_copied]
    );
    assert!(external_readonly.as_slice()[external_copied..]
        .iter()
        .all(|byte| *byte == 0));
    assert!(external_readonly
        .padding_slice()
        .iter()
        .all(|byte| *byte == 0));
    assert_eq!(external_original.as_slice(), payload.as_slice());
    assert!(external_original.is_readonly());
    assert_eq!(external_original.opaque_ref::<usize>(), Some(&payload_len));
    assert!(std::ptr::eq(
        external_original.as_padded_slice().as_ptr(),
        external_storage.as_ptr()
    ));
    drop(external_readonly);
    assert!(external_released.lock().unwrap().is_empty());
    drop(external_original);
    assert_eq!(*external_released.lock().unwrap(), vec![payload_len]);
    assert_eq!(Arc::strong_count(&external_storage), 1);
    let invalid_external_released = Arc::new(Mutex::new(Vec::<usize>::new()));
    let invalid_external_capture = Arc::clone(&invalid_external_released);
    assert_eq!(
        BufferRef::from_external_slice_with_len_and_opaque_readonly(
            Arc::clone(&external_storage),
            external_storage.len().saturating_add(1),
            payload_len,
            move |opaque| {
                invalid_external_capture.lock().unwrap().push(opaque);
            },
        )
        .unwrap_err()
        .kind(),
        AvErrorKind::InvalidArgument
    );
    assert!(invalid_external_released.lock().unwrap().is_empty());
    assert_eq!(Arc::strong_count(&external_storage), 1);

    let pool = BufferPool::new(payload_len, padding_len).unwrap();
    assert_eq!(pool.len(), payload_len);
    assert_eq!(pool.allocated_len(), payload_len + padding_len);
    assert_eq!(pool.padding_len(), padding_len);
    assert_eq!(pool.available_count().unwrap(), 0);

    let mut pooled = pool.get().unwrap();
    assert_eq!(pooled.len(), payload_len);
    assert_eq!(pooled.allocated_len(), payload_len + padding_len);
    assert!(pooled.as_padded_slice().iter().all(|byte| *byte == 0));
    if !pooled.is_empty() {
        pooled.make_mut()[0] = cursor.next().unwrap_or_default();
    }
    let shared = pooled.clone();
    assert_eq!(
        pool.recycle(pooled).unwrap_err().kind(),
        AvErrorKind::InvalidArgument
    );
    assert_eq!(pool.available_count().unwrap(), 0);
    drop(shared);

    let wrong_shape =
        BufferRef::zeroed_with_padding(payload_len.saturating_add(1), padding_len).unwrap();
    assert_eq!(
        pool.recycle(wrong_shape).unwrap_err().kind(),
        AvErrorKind::InvalidArgument
    );
    let mut recyclable = pool.get().unwrap();
    if !recyclable.is_empty() {
        let last = recyclable.len() - 1;
        recyclable.make_mut()[last] = cursor.next().unwrap_or_default();
    }
    pool.recycle(recyclable).unwrap();
    assert_eq!(pool.available_count().unwrap(), 1);
    let reused = pool.get().unwrap();
    assert_eq!(pool.available_count().unwrap(), 0);
    assert_eq!(reused.len(), payload_len);
    assert_eq!(reused.allocated_len(), payload_len + padding_len);
    assert!(reused.as_padded_slice().iter().all(|byte| *byte == 0));
    drop(reused);
    assert_eq!(pool.available_count().unwrap(), 1);

    let auto_reused = pool.get().unwrap();
    assert_eq!(pool.available_count().unwrap(), 0);
    assert!(auto_reused.as_padded_slice().iter().all(|byte| *byte == 0));
    let auto_shared = auto_reused.clone();
    drop(auto_reused);
    assert_eq!(pool.available_count().unwrap(), 0);
    drop(auto_shared);
    assert_eq!(pool.available_count().unwrap(), 1);

    let cow_pool = BufferPool::new(payload_len, padding_len).unwrap();
    let mut cow_buffer = cow_pool.get().unwrap();
    let cow_shared = cow_buffer.clone();
    if !cow_buffer.is_empty() {
        cow_buffer.make_mut()[0] = cursor.next().unwrap_or_default();
    } else {
        assert_eq!(cow_buffer.make_mut(), &mut []);
    }
    drop(cow_buffer);
    assert_eq!(cow_pool.available_count().unwrap(), 0);
    drop(cow_shared);
    assert_eq!(cow_pool.available_count().unwrap(), 1);

    let release_storage = {
        let mut storage = payload.clone();
        storage.resize(payload.len() + padding_len, 0);
        storage
    };
    let released = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
    let capture = Arc::clone(&released);
    let callback_buffer = BufferRef::from_vec_with_len_and_release_callback(
        release_storage.clone(),
        payload.len(),
        move |storage| {
            capture.lock().unwrap().push(storage);
        },
    )
    .unwrap();
    assert_eq!(callback_buffer.as_slice(), payload.as_slice());
    assert_eq!(callback_buffer.padding_len(), padding_len);
    let callback_slice = callback_buffer.slice(0, callback_buffer.len()).unwrap();
    drop(callback_buffer);
    assert!(released.lock().unwrap().is_empty());
    drop(callback_slice);
    assert_eq!(*released.lock().unwrap(), vec![release_storage.clone()]);

    let rejected_release_count = Arc::new(Mutex::new(0usize));
    let rejected_capture = Arc::clone(&rejected_release_count);
    assert_eq!(
        BufferRef::from_vec_with_len_and_release_callback(Vec::new(), 1, move |_| {
            *rejected_capture.lock().unwrap() += 1;
        })
        .unwrap_err()
        .kind(),
        AvErrorKind::InvalidArgument
    );
    assert_eq!(*rejected_release_count.lock().unwrap(), 0);

    let cow_released = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
    let cow_capture = Arc::clone(&cow_released);
    let mut callback_cow =
        BufferRef::from_vec_with_release_callback(payload.clone(), move |storage| {
            cow_capture.lock().unwrap().push(storage);
        });
    let callback_cow_shared = callback_cow.clone();
    if !callback_cow.is_empty() {
        callback_cow.make_mut()[0] = cursor.next().unwrap_or_default();
    } else {
        assert_eq!(callback_cow.make_mut(), &mut []);
    }
    drop(callback_cow);
    assert!(cow_released.lock().unwrap().is_empty());
    drop(callback_cow_shared);
    assert_eq!(*cow_released.lock().unwrap(), vec![payload.clone()]);

    let mut readonly =
        BufferRef::from_vec_with_len_readonly(release_storage.clone(), payload.len()).unwrap();
    let readonly_shared = readonly.clone();
    assert!(readonly.is_readonly());
    assert!(!readonly.is_writable());
    assert!(readonly.get_mut().is_none());
    assert_eq!(readonly.as_slice(), payload.as_slice());
    assert_eq!(readonly.padding_len(), padding_len);
    if !readonly.is_empty() {
        readonly.make_mut()[0] = cursor.next().unwrap_or_default();
    } else {
        assert_eq!(readonly.make_mut(), &mut []);
    }
    assert!(!readonly.is_readonly());
    assert!(readonly.is_writable());
    assert_eq!(
        readonly_shared.as_padded_slice(),
        release_storage.as_slice()
    );
    assert!(readonly_shared.is_readonly());
    drop(readonly);
    drop(readonly_shared);

    let readonly_released = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
    let readonly_capture = Arc::clone(&readonly_released);
    let mut readonly_callback = BufferRef::from_vec_with_len_and_release_callback_readonly(
        release_storage.clone(),
        payload.len(),
        move |storage| {
            readonly_capture.lock().unwrap().push(storage);
        },
    )
    .unwrap();
    assert!(readonly_callback.is_readonly());
    readonly_callback.make_mut();
    assert_eq!(*readonly_released.lock().unwrap(), vec![release_storage]);
    assert!(!readonly_callback.is_readonly());
    assert!(readonly_callback.is_writable());

    let custom_allocations = Arc::new(Mutex::new(Vec::<usize>::new()));
    let custom_releases = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
    let allocate_capture = Arc::clone(&custom_allocations);
    let release_capture = Arc::clone(&custom_releases);
    let custom_pool = BufferPool::with_callbacks(
        payload_len,
        padding_len,
        BufferPoolCallbacks::new(
            move |allocated_len| {
                allocate_capture.lock().unwrap().push(allocated_len);
                Ok(vec![0xaa; allocated_len])
            },
            move |storage| {
                release_capture.lock().unwrap().push(storage);
            },
        ),
    )
    .unwrap();
    let custom_buffer = custom_pool.get().unwrap();
    assert_eq!(
        *custom_allocations.lock().unwrap(),
        vec![payload_len + padding_len]
    );
    assert!(custom_buffer
        .as_padded_slice()
        .iter()
        .all(|byte| *byte == 0));
    drop(custom_pool);
    assert!(custom_releases.lock().unwrap().is_empty());
    drop(custom_buffer);
    assert_eq!(
        *custom_releases.lock().unwrap(),
        vec![vec![0; payload_len + padding_len]]
    );

    let bad_releases = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
    let bad_release_capture = Arc::clone(&bad_releases);
    let bad_pool = BufferPool::with_callbacks(
        payload_len,
        padding_len,
        BufferPoolCallbacks::new(
            |allocated_len| Ok(vec![1; allocated_len + 1]),
            move |storage| {
                bad_release_capture.lock().unwrap().push(storage);
            },
        ),
    )
    .unwrap();
    assert_eq!(
        bad_pool.get().unwrap_err().kind(),
        AvErrorKind::InvalidArgument
    );
    assert_eq!(
        *bad_releases.lock().unwrap(),
        vec![vec![1; payload_len + padding_len + 1]]
    );
    assert_eq!(
        BufferPool::new(1, usize::MAX).unwrap_err().kind(),
        AvErrorKind::InvalidArgument
    );
}

fn exercise_errors(cursor: &mut Cursor<'_>) {
    let io_kind = io_error_kind_from(cursor.next());
    let err = AvError::from_io_error("fuzz io", io::Error::new(io_kind, "source failure"));
    assert_eq!(err.kind(), expected_av_error_kind_for_io(io_kind));
    assert_eq!(err.io_kind(), Some(io_kind));
    assert_eq!(err.is_eof(), err.kind() == AvErrorKind::EndOfFile);
    assert!(err.message().contains("fuzz io"));
    assert!(err.message().contains("source failure"));

    for (err, kind) in [
        (
            AvError::invalid_argument("invalid argument"),
            AvErrorKind::InvalidArgument,
        ),
        (
            AvError::invalid_data("invalid data"),
            AvErrorKind::InvalidData,
        ),
        (AvError::not_found("not found"), AvErrorKind::NotFound),
        (AvError::end_of_file("end of file"), AvErrorKind::EndOfFile),
        (
            AvError::unsupported("unsupported"),
            AvErrorKind::Unsupported,
        ),
        (AvError::external("external"), AvErrorKind::External),
        (AvError::bug("bug"), AvErrorKind::Bug),
    ] {
        assert_eq!(err.kind(), kind);
        assert_eq!(err.io_kind(), None);
        assert_eq!(err.is_eof(), kind == AvErrorKind::EndOfFile);
        assert!(!err.message().is_empty());
    }
}

fn exercise_rational_and_timebase(cursor: &mut Cursor<'_>) {
    let numerator = small_i32_from(cursor.next());
    let denominator = small_i32_from(cursor.next());
    let Ok(rational) = Rational::new(numerator, denominator) else {
        assert_eq!(denominator, 0);
        return;
    };

    assert!(rational.den() > 0);
    if numerator == 0 {
        assert_eq!(rational, Rational::ZERO);
        assert!(rational.reciprocal().is_err());
    } else {
        let reciprocal = rational.reciprocal().unwrap();
        assert_eq!(rational.checked_mul(reciprocal).unwrap(), Rational::ONE);
    }

    let other = positive_rational_from(cursor.next(), cursor.next());
    assert_eq!(
        rational
            .checked_add(other)
            .unwrap()
            .checked_sub(other)
            .unwrap(),
        rational
    );
    assert_eq!(
        rational
            .checked_sub(other)
            .unwrap()
            .checked_add(other)
            .unwrap(),
        rational
    );
    assert_eq!(
        rational.checked_neg().unwrap().checked_neg().unwrap(),
        rational
    );
    assert_eq!(
        rational
            .checked_div(other)
            .unwrap()
            .checked_mul(other)
            .unwrap(),
        rational
    );

    let src = positive_rational_from(cursor.next(), cursor.next());
    let dst = positive_rational_from(cursor.next(), cursor.next());
    let value = small_i64_from(cursor.next(), cursor.next());
    assert_eq!(
        rescale_q(value, src, dst).unwrap(),
        expected_rescale(value, src, dst, Rounding::NearInf).unwrap()
    );
    for rounding in [
        Rounding::Zero,
        Rounding::Inf,
        Rounding::Down,
        Rounding::Up,
        Rounding::NearInf,
    ] {
        assert_eq!(
            rescale_q_rnd(value, src, dst, rounding).unwrap(),
            expected_rescale(value, src, dst, rounding).unwrap()
        );
    }
    for sentinel in [i64::MIN, i64::MAX] {
        assert_eq!(
            rescale_q_rnd_pass_minmax(sentinel, src, dst, Rounding::NearInf).unwrap(),
            sentinel
        );
    }
    assert_eq!(
        rescale_q_rnd_pass_minmax(value, src, dst, Rounding::NearInf).unwrap(),
        expected_rescale(value, src, dst, Rounding::NearInf).unwrap()
    );
}

fn exercise_pixel_and_video_frame(cursor: &mut Cursor<'_>) {
    let pixel_format = pixel_format_from(cursor.next());
    let width = dimension_from(cursor.next());
    let height = dimension_from(cursor.next());

    let Ok(plane_sizes) = pixel_format.plane_sizes(width, height) else {
        assert!(width == 0 || height == 0 || pixel_format == PixelFormat::Yuv420p);
        return;
    };
    let frame_size = pixel_format.frame_size(width, height).unwrap();
    assert_eq!(frame_size, plane_sizes.iter().sum::<usize>());
    assert_eq!(pixel_format.plane_count(), plane_sizes.len());
    assert_eq!(pixel_format.is_planar(), pixel_format.plane_count() > 1);

    let payload = payload_from(cursor, frame_size);
    let planes = pixel_format.split_planes(&payload, width, height).unwrap();
    assert_eq!(planes.len(), plane_sizes.len());
    assert_eq!(planes.iter().map(Vec::len).collect::<Vec<_>>(), plane_sizes);
    assert_eq!(planes.concat(), payload);

    let video = VideoFrame::new(width, height, pixel_format, planes.clone()).unwrap();
    assert_eq!(video.width(), width);
    assert_eq!(video.height(), height);
    assert_eq!(video.pixel_format(), pixel_format);
    assert_eq!(video.pixel_format_name(), pixel_format.name());
    assert_eq!(
        video.line_sizes(),
        expected_video_line_sizes(pixel_format, width).as_slice()
    );
    assert_eq!(video.planes(), planes.as_slice());
    assert_eq!(video.plane_buffers().len(), planes.len());
    for (plane_buffer, plane) in video.plane_buffers().iter().zip(&planes) {
        assert_eq!(plane_buffer.as_slice(), plane.as_slice());
    }

    let video_plane_buffers = planes
        .iter()
        .map(|plane| BufferRef::copy_from_slice_with_padding(plane, 1).unwrap())
        .collect::<Vec<_>>();
    let video_from_buffers =
        VideoFrame::new_with_buffer_refs(width, height, pixel_format, video_plane_buffers.clone())
            .unwrap();
    assert_eq!(video_from_buffers.planes(), planes.as_slice());
    for (stored, source) in video_from_buffers
        .plane_buffers()
        .iter()
        .zip(&video_plane_buffers)
    {
        assert!(stored.shares_storage(source));
        assert_eq!(stored.padding_slice(), &[0]);
    }

    let video_plane_shapes = expected_video_plane_shapes(pixel_format, width, height);
    let strided_line_sizes = video_plane_shapes
        .iter()
        .map(|(row_bytes, _)| row_bytes + 1)
        .collect::<Vec<_>>();
    let strided_planes = planes
        .iter()
        .zip(&video_plane_shapes)
        .zip(&strided_line_sizes)
        .map(|((plane, &(row_bytes, rows)), &line_size)| {
            let mut strided = Vec::with_capacity(line_size * rows);
            for row in 0..rows {
                let start = row * row_bytes;
                let end = start + row_bytes;
                strided.extend_from_slice(&plane[start..end]);
                strided.resize(strided.len() + (line_size - row_bytes), 0xEE);
            }
            strided
        })
        .collect::<Vec<_>>();
    let strided_video = VideoFrame::new_with_line_sizes(
        width,
        height,
        pixel_format,
        strided_planes.clone(),
        strided_line_sizes.clone(),
    )
    .unwrap();
    assert_eq!(strided_video.line_sizes(), strided_line_sizes.as_slice());
    assert_eq!(strided_video.planes(), planes.as_slice());
    for (stored, strided) in strided_video.plane_buffers().iter().zip(&strided_planes) {
        assert_eq!(stored.as_slice(), strided.as_slice());
    }
    let video_alignment = usize::from(cursor.next().unwrap_or_default() % 8) + 1;
    let aligned_video_line_sizes =
        VideoFrame::aligned_line_sizes(pixel_format, width, height, video_alignment).unwrap();
    for (line_size, (row_bytes, _)) in aligned_video_line_sizes.iter().zip(&video_plane_shapes) {
        assert!(*line_size >= *row_bytes);
        assert_eq!(line_size % video_alignment, 0);
    }
    let aligned_video_planes = planes
        .iter()
        .zip(&video_plane_shapes)
        .zip(&aligned_video_line_sizes)
        .map(|((plane, &(row_bytes, rows)), &line_size)| {
            let mut aligned = Vec::with_capacity(line_size * rows);
            for row in 0..rows {
                let start = row * row_bytes;
                let end = start + row_bytes;
                aligned.extend_from_slice(&plane[start..end]);
                aligned.resize(aligned.len() + (line_size - row_bytes), 0xDD);
            }
            aligned
        })
        .collect::<Vec<_>>();
    let aligned_video = VideoFrame::new_with_aligned_line_sizes(
        width,
        height,
        pixel_format,
        aligned_video_planes.clone(),
        video_alignment,
    )
    .unwrap();
    assert_eq!(
        aligned_video.line_sizes(),
        aligned_video_line_sizes.as_slice()
    );
    assert_eq!(aligned_video.planes(), planes.as_slice());
    let aligned_video_buffers = aligned_video_planes
        .iter()
        .map(|plane| BufferRef::copy_from_slice(plane))
        .collect::<Vec<_>>();
    let aligned_video_from_buffers = VideoFrame::new_with_buffer_refs_and_aligned_line_sizes(
        width,
        height,
        pixel_format,
        aligned_video_buffers.clone(),
        video_alignment,
    )
    .unwrap();
    assert_eq!(aligned_video_from_buffers.planes(), planes.as_slice());
    for (stored, source) in aligned_video_from_buffers
        .plane_buffers()
        .iter()
        .zip(&aligned_video_buffers)
    {
        assert!(stored.shares_storage(source));
    }
    assert_eq!(
        VideoFrame::aligned_line_sizes(pixel_format, width, height, 0)
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidArgument
    );
    let mut undersized_line_sizes = strided_line_sizes.clone();
    undersized_line_sizes[0] = video_plane_shapes[0].0.saturating_sub(1);
    assert_eq!(
        VideoFrame::new_with_line_sizes(
            width,
            height,
            pixel_format,
            strided_planes.clone(),
            undersized_line_sizes
        )
        .unwrap_err()
        .kind(),
        AvErrorKind::InvalidArgument
    );

    let mut mutable_strided_video = strided_video.clone();
    let shared_strided_video = mutable_strided_video.clone();
    assert!(!mutable_strided_video.is_writable());
    let replacement_planes = planes
        .iter()
        .map(|plane| {
            plane
                .iter()
                .map(|byte| byte.wrapping_add(1))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for (index, replacement) in replacement_planes.iter().enumerate() {
        mutable_strided_video
            .set_plane_visible_data(index, replacement)
            .unwrap();
    }
    assert!(mutable_strided_video.is_writable());
    assert_eq!(
        mutable_strided_video.planes(),
        replacement_planes.as_slice()
    );
    for ((((stored, shared), original_strided), replacement), (shape, line_size)) in
        mutable_strided_video
            .plane_buffers()
            .iter()
            .zip(shared_strided_video.plane_buffers())
            .zip(&strided_planes)
            .zip(&replacement_planes)
            .zip(video_plane_shapes.iter().zip(&strided_line_sizes))
    {
        let (row_bytes, rows) = *shape;
        let line_size = *line_size;
        assert!(!stored.shares_storage(shared));
        let mut expected_strided = original_strided.clone();
        for row in 0..rows {
            let visible_start = row * row_bytes;
            let visible_end = visible_start + row_bytes;
            let storage_start = row * line_size;
            let storage_end = storage_start + row_bytes;
            expected_strided[storage_start..storage_end]
                .copy_from_slice(&replacement[visible_start..visible_end]);
        }
        assert_eq!(stored.as_slice(), expected_strided.as_slice());
    }
    assert_eq!(
        mutable_strided_video
            .set_plane_visible_data(replacement_planes.len(), &[])
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidArgument
    );
    assert_eq!(
        mutable_strided_video
            .set_plane_visible_data(0, &replacement_planes[0][..replacement_planes[0].len() - 1])
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let mut frame = Frame::video(video);
    let pts = timestamp_from(cursor.next());
    frame.set_pts(pts);
    assert_eq!(frame.pts(), pts);
    assert!(matches!(frame.data(), FrameData::Video(_)));
    assert!(frame.hw_frames_context().is_none());
    assert!(frame.side_data().is_empty());
    assert!(frame.is_writable());
    let shared_frame_payload = frame.clone();
    assert!(!frame.is_writable());
    frame.make_writable();
    assert!(frame.is_writable());
    let (frame_video, shared_video) = match (frame.data(), shared_frame_payload.data()) {
        (FrameData::Video(frame_video), FrameData::Video(shared_video)) => {
            (frame_video, shared_video)
        }
        _ => unreachable!("constructed video frame changed variant"),
    };
    assert_eq!(frame_video.planes(), planes.as_slice());
    for (stored, shared) in frame_video
        .plane_buffers()
        .iter()
        .zip(shared_video.plane_buffers())
    {
        assert!(!stored.shares_storage(shared));
    }
    let frame_replacement = planes[0]
        .iter()
        .map(|byte| byte.wrapping_add(2))
        .collect::<Vec<_>>();
    frame.set_plane_visible_data(0, &frame_replacement).unwrap();
    let (frame_video, shared_video) = match (frame.data(), shared_frame_payload.data()) {
        (FrameData::Video(frame_video), FrameData::Video(shared_video)) => {
            (frame_video, shared_video)
        }
        _ => unreachable!("constructed video frame changed variant"),
    };
    assert_eq!(frame_video.planes()[0], frame_replacement);
    assert_eq!(shared_video.planes(), planes.as_slice());

    let frame_side_data_len = usize::from(cursor.next().unwrap_or_default() % 48);
    let frame_side_data_payload = payload_from(cursor, frame_side_data_len);
    let frame_side_data_buffer =
        BufferRef::copy_from_slice_with_padding(&frame_side_data_payload, 1).unwrap();
    let frame_side_data_kind = frame_side_data_kind_from(cursor.next());
    let mut frame_side_data = FrameSideData::new_with_kind_and_buffer_ref(
        frame_side_data_kind.clone(),
        frame_side_data_buffer.clone(),
    )
    .unwrap();
    frame_side_data
        .metadata_mut()
        .set("origin", "fuzz")
        .unwrap();
    frame.push_side_data(frame_side_data);
    assert_eq!(frame.side_data().len(), 1);
    assert_eq!(frame.side_data()[0].kind(), frame_side_data_kind.name());
    assert_eq!(frame.side_data()[0].kind_id(), &frame_side_data_kind);
    assert_eq!(
        frame.side_data()[0].is_known_kind(),
        frame_side_data_kind.is_known()
    );
    assert_eq!(
        frame.side_data()[0].descriptor(),
        frame_side_data_kind.descriptor()
    );
    assert_eq!(
        frame.side_data()[0].properties(),
        frame_side_data_kind.properties()
    );
    assert_eq!(
        frame.side_data()[0].supports_multiple_instances(),
        frame_side_data_kind.supports_multiple_instances()
    );
    match frame.side_data()[0].display_matrix() {
        Ok(Some(value)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::DisplayMatrix);
            assert_eq!(frame_side_data_payload.len(), FrameDisplayMatrix::DATA_LEN);
            assert_eq!(value.to_bytes().as_slice(), frame_side_data_payload);
            assert_eq!(FrameDisplayMatrix::new(value.elements()), value);
            assert_eq!(
                FrameDisplayMatrix::parse(value.to_bytes().as_slice()).unwrap(),
                value
            );
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::DisplayMatrix),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::DisplayMatrix);
            assert_ne!(frame_side_data_payload.len(), FrameDisplayMatrix::DATA_LEN);
        }
    }
    match frame.side_data()[0].matrix_encoding() {
        Ok(Some(value)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::MatrixEncoding);
            assert_eq!(frame_side_data_payload.len(), FrameMatrixEncoding::DATA_LEN);
            let mut raw = [0; FrameMatrixEncoding::DATA_LEN];
            raw.copy_from_slice(&frame_side_data_payload);
            assert_eq!(i32::from_ne_bytes(raw), value.as_raw());
            assert_eq!(value.to_bytes().as_slice(), frame_side_data_payload);
            assert_eq!(
                FrameMatrixEncoding::from_raw(value.as_raw()).unwrap(),
                value
            );
            assert_eq!(
                FrameMatrixEncoding::parse(value.to_bytes().as_slice()).unwrap(),
                value
            );
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::MatrixEncoding),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::MatrixEncoding);
            let raw_invalid = if frame_side_data_payload.len() == FrameMatrixEncoding::DATA_LEN {
                let mut raw = [0; FrameMatrixEncoding::DATA_LEN];
                raw.copy_from_slice(&frame_side_data_payload);
                FrameMatrixEncoding::from_raw(i32::from_ne_bytes(raw)).is_err()
            } else {
                false
            };
            assert!(frame_side_data_payload.len() != FrameMatrixEncoding::DATA_LEN || raw_invalid);
        }
    }
    match frame.side_data()[0].downmix_info() {
        Ok(Some(value)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::DownmixInfo);
            assert_eq!(frame_side_data_payload.len(), FrameDownmixInfo::DATA_LEN);
            let mut raw_type = [0; 4];
            raw_type.copy_from_slice(&frame_side_data_payload[..4]);
            assert_eq!(
                i32::from_ne_bytes(raw_type),
                value.preferred_downmix_type().as_raw()
            );
            assert_eq!(
                FrameDownmixType::from_raw(value.preferred_downmix_type().as_raw()).unwrap(),
                value.preferred_downmix_type()
            );
            for (bits, chunk) in value
                .level_bits()
                .iter()
                .zip(frame_side_data_payload[8..].chunks_exact(8))
            {
                let mut raw = [0; 8];
                raw.copy_from_slice(chunk);
                assert_eq!(*bits, u64::from_ne_bytes(raw));
            }
            assert_eq!(FrameDownmixInfo::parse(&value.to_bytes()).unwrap(), value);
            assert_eq!(&value.to_bytes()[4..8], &[0, 0, 0, 0]);
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::DownmixInfo),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::DownmixInfo);
            let raw_invalid = if frame_side_data_payload.len() == FrameDownmixInfo::DATA_LEN {
                let mut raw = [0; 4];
                raw.copy_from_slice(&frame_side_data_payload[..4]);
                FrameDownmixType::from_raw(i32::from_ne_bytes(raw)).is_err()
            } else {
                false
            };
            assert!(frame_side_data_payload.len() != FrameDownmixInfo::DATA_LEN || raw_invalid);
        }
    }
    match frame.side_data()[0].active_format_description() {
        Ok(Some(value)) => {
            assert_eq!(
                frame_side_data_kind,
                FrameSideDataKind::ActiveFormatDescription
            );
            assert_eq!(frame_side_data_payload, vec![value.as_byte()]);
            assert!(FrameActiveFormatDescription::from_byte(value.as_byte()).is_ok());
        }
        Ok(None) => assert_ne!(
            frame_side_data_kind,
            FrameSideDataKind::ActiveFormatDescription
        ),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(
                frame_side_data_kind,
                FrameSideDataKind::ActiveFormatDescription
            );
            assert!(
                frame_side_data_payload.len() != FrameActiveFormatDescription::DATA_LEN
                    || FrameActiveFormatDescription::from_byte(frame_side_data_payload[0]).is_err()
            );
        }
    }
    match frame.side_data()[0].skip_samples() {
        Ok(Some(payload)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::SkipSamples);
            let payload_bytes = payload.to_bytes();
            assert_eq!(&payload_bytes[..], frame_side_data_payload.as_slice());
            assert_eq!(
                FrameSkipSamplesReason::from_byte(payload.start_reason().as_byte()).unwrap(),
                payload.start_reason()
            );
            assert_eq!(
                FrameSkipSamplesReason::from_byte(payload.end_reason().as_byte()).unwrap(),
                payload.end_reason()
            );
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::SkipSamples),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::SkipSamples);
            assert!(
                frame_side_data_payload.len() != FrameSkipSamples::DATA_LEN
                    || FrameSkipSamplesReason::from_byte(frame_side_data_payload[8]).is_err()
                    || FrameSkipSamplesReason::from_byte(frame_side_data_payload[9]).is_err()
            );
        }
    }
    match frame.side_data()[0].audio_service_type() {
        Ok(Some(value)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::AudioServiceType);
            assert!(frame_side_data_payload.len() >= FrameAudioServiceType::DATA_LEN);
            let mut raw = [0; FrameAudioServiceType::DATA_LEN];
            raw.copy_from_slice(&frame_side_data_payload[..FrameAudioServiceType::DATA_LEN]);
            assert_eq!(i32::from_ne_bytes(raw), value.as_raw());
            assert_eq!(
                FrameAudioServiceType::from_raw(value.as_raw()).unwrap(),
                value
            );
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::AudioServiceType),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::AudioServiceType);
            let raw_invalid = if frame_side_data_payload.len() >= FrameAudioServiceType::DATA_LEN {
                let mut raw = [0; FrameAudioServiceType::DATA_LEN];
                raw.copy_from_slice(&frame_side_data_payload[..FrameAudioServiceType::DATA_LEN]);
                FrameAudioServiceType::from_raw(i32::from_ne_bytes(raw)).is_err()
            } else {
                false
            };
            assert!(frame_side_data_payload.len() < FrameAudioServiceType::DATA_LEN || raw_invalid);
        }
    }
    match frame.side_data()[0].gop_timecode() {
        Ok(Some(value)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::GopTimecode);
            assert_eq!(frame_side_data_payload.len(), FrameGopTimecode::DATA_LEN);
            assert_eq!(value.to_bytes().as_slice(), frame_side_data_payload);
            assert!((0..=FrameGopTimecode::MAX_VALUE).contains(&value.as_raw_i64()));
            assert_eq!(
                FrameGopTimecode::from_raw_i64(value.as_raw_i64()).unwrap(),
                value
            );
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::GopTimecode),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::GopTimecode);
            let raw_invalid = if frame_side_data_payload.len() == FrameGopTimecode::DATA_LEN {
                let mut raw = [0; FrameGopTimecode::DATA_LEN];
                raw.copy_from_slice(&frame_side_data_payload[..FrameGopTimecode::DATA_LEN]);
                FrameGopTimecode::from_raw_i64(i64::from_ne_bytes(raw)).is_err()
            } else {
                false
            };
            assert!(frame_side_data_payload.len() != FrameGopTimecode::DATA_LEN || raw_invalid);
        }
    }
    match frame.side_data()[0].s12m_timecode() {
        Ok(Some(payload)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::S12mTimecode);
            assert_eq!(frame_side_data_payload.len(), FrameS12mTimecode::DATA_LEN);
            assert_eq!(payload.to_bytes().as_slice(), frame_side_data_payload);
            assert!(
                (FrameS12mTimecode::MIN_TIMECODES..=FrameS12mTimecode::MAX_TIMECODES)
                    .contains(&payload.count())
            );
            assert_eq!(payload.timecodes().len(), payload.count());
            assert_eq!(
                FrameS12mTimecode::from_raw_words(payload.raw_words()).unwrap(),
                payload
            );
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::S12mTimecode),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::S12mTimecode);
            let raw_count_invalid = if frame_side_data_payload.len() == FrameS12mTimecode::DATA_LEN
            {
                let mut raw = [0; 4];
                raw.copy_from_slice(&frame_side_data_payload[..4]);
                !matches!(u32::from_ne_bytes(raw), 1..=3)
            } else {
                false
            };
            assert!(
                frame_side_data_payload.len() != FrameS12mTimecode::DATA_LEN || raw_count_invalid
            );
        }
    }
    match frame.side_data()[0].sei_unregistered() {
        Ok(Some(payload)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::SeiUnregistered);
            assert_eq!(
                payload.uuid().as_slice(),
                &frame_side_data_payload[..FrameSeiUnregistered::UUID_LEN]
            );
            assert_eq!(
                payload.user_data(),
                &frame_side_data_payload[FrameSeiUnregistered::UUID_LEN..]
            );
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::SeiUnregistered),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::SeiUnregistered);
            assert!(frame_side_data_payload.len() < FrameSeiUnregistered::UUID_LEN);
        }
    }
    assert_eq!(
        frame.side_data()[0].data(),
        frame_side_data_payload.as_slice()
    );
    assert_eq!(frame.side_data()[0].metadata().get("origin"), Some("fuzz"));
    assert!(frame.side_data()[0]
        .buffer()
        .shares_storage(&frame_side_data_buffer));
    assert_eq!(frame.side_data()[0].buffer().padding_slice(), &[0]);
    assert_eq!(
        frame
            .side_data_by_kind(&frame_side_data_kind)
            .unwrap()
            .data(),
        frame_side_data_payload.as_slice()
    );
    frame
        .add_side_data_kind_buffer(
            frame_side_data_kind.clone(),
            BufferRef::copy_from_slice(&[0xAA]),
        )
        .unwrap();
    assert_eq!(frame.side_data().len(), 2);
    let replacement_side_data_buffer =
        BufferRef::copy_from_slice_with_padding(&[0x55, 0x66], 1).unwrap();
    let replaced_side_data = frame
        .set_side_data_kind_buffer(
            frame_side_data_kind.clone(),
            replacement_side_data_buffer.clone(),
        )
        .unwrap();
    assert_eq!(replaced_side_data.len(), 2);
    assert!(replaced_side_data
        .iter()
        .all(|side_data| side_data.kind_id() == &frame_side_data_kind));
    assert!(replaced_side_data[0]
        .buffer()
        .shares_storage(&frame_side_data_buffer));
    assert_eq!(replaced_side_data[1].data(), &[0xAA]);
    assert_eq!(frame.side_data().len(), 1);
    assert_eq!(frame.side_data()[0].data(), &[0x55, 0x66]);
    assert!(frame.side_data()[0]
        .buffer()
        .shares_storage(&replacement_side_data_buffer));
    frame
        .side_data_by_kind_mut(&frame_side_data_kind)
        .unwrap()
        .metadata_mut()
        .set("origin", "replacement")
        .unwrap();
    assert_eq!(
        frame
            .side_data_by_kind(&frame_side_data_kind)
            .unwrap()
            .metadata()
            .get("origin"),
        Some("replacement")
    );

    let removed_side_data = frame.remove_side_data_kind(&frame_side_data_kind).unwrap();
    assert!(frame.side_data().is_empty());
    assert!(removed_side_data
        .buffer()
        .shares_storage(&replacement_side_data_buffer));
    frame.push_side_data(removed_side_data);
    assert!(frame.remove_side_data("missing").is_none());
    assert_eq!(
        FrameSideDataKind::from_name("Display Matrix").unwrap(),
        FrameSideDataKind::DisplayMatrix
    );
    assert_eq!(
        FrameSideDataKind::from_name("AV_FRAME_DATA_GOP_TIMECODE").unwrap(),
        FrameSideDataKind::GopTimecode
    );
    assert_eq!(
        FrameSideDataKind::from_name("3D Reference Displays").unwrap(),
        FrameSideDataKind::ThreeDReferenceDisplays
    );
    assert_eq!(
        FrameSideDataKind::ThreeDReferenceDisplays.ffmpeg_constant(),
        Some("AV_FRAME_DATA_3D_REFERENCE_DISPLAYS")
    );
    assert!(FrameSideDataKind::ThreeDReferenceDisplays
        .properties()
        .contains(FrameSideDataProperties::GLOBAL));
    assert_eq!(
        FrameSideDataKind::MotionVectors.descriptor_name(),
        Some("Motion vectors")
    );
    assert!(FrameSideDataKind::MotionVectors
        .properties()
        .contains(FrameSideDataProperties::SIZE_DEPENDENT));
    assert!(FrameSideDataKind::SeiUnregistered.supports_multiple_instances());
    assert_eq!(
        FrameSideDataProperties::from_bits_truncate(u32::MAX).bits(),
        FrameSideDataProperties::ALL.bits()
    );
    assert_eq!(FrameSideDataKind::KNOWN.len(), 32);
    assert_eq!(
        FrameSideDataKind::from_name("vendor.private.side-data")
            .unwrap()
            .name(),
        "vendor.private.side-data"
    );
    assert_eq!(
        FrameSideDataKind::Unknown(String::from("vendor.private.side-data")).ffmpeg_constant(),
        None
    );
    assert_eq!(
        FrameSideDataKind::Unknown(String::from("vendor.private.side-data")).descriptor(),
        None
    );
    assert_eq!(
        FrameSideData::new(" \t", Vec::new()).unwrap_err().kind(),
        AvErrorKind::InvalidArgument
    );
    assert_eq!(
        FrameSideData::new("bad\0kind", Vec::new())
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidArgument
    );

    let context_payload = vec![cursor.next().unwrap_or_default()];
    let released_contexts = Arc::new(Mutex::new(Vec::<Vec<u8>>::new()));
    let release_capture = Arc::clone(&released_contexts);
    let context = BufferRef::from_external_slice_with_opaque_readonly(
        Arc::<[u8]>::from(context_payload.clone()),
        context_payload.clone(),
        move |opaque| {
            release_capture.lock().unwrap().push(opaque);
        },
    );
    frame.set_hw_frames_context(Some(context.clone()));
    assert!(frame.hw_frames_context().unwrap().shares_storage(&context));
    assert_eq!(
        frame.hw_frames_context().unwrap().opaque_ref::<Vec<u8>>(),
        Some(&context_payload)
    );
    let cloned_frame = frame.clone();
    drop(context);
    assert!(released_contexts.lock().unwrap().is_empty());
    let taken_context = frame.take_hw_frames_context().unwrap();
    assert!(frame.hw_frames_context().is_none());
    drop(taken_context);
    assert!(released_contexts.lock().unwrap().is_empty());
    drop(cloned_frame);
    assert_eq!(*released_contexts.lock().unwrap(), vec![context_payload]);

    if frame_size > 0 {
        let err = pixel_format
            .split_planes(&payload[..frame_size - 1], width, height)
            .unwrap_err();
        assert_eq!(err.kind(), AvErrorKind::InvalidData);
    }
}

fn exercise_sample_channel_and_audio_frame(cursor: &mut Cursor<'_>) {
    let sample_rate = sample_rate_from(cursor.next());
    let channels = channel_count_from(cursor.next());
    let samples_per_channel = usize::from(cursor.next().unwrap_or_default()) % (MAX_SAMPLES + 1);
    let sample_format = sample_format_from(cursor.next());

    assert_eq!(
        SampleFormat::from_name(sample_format.name()),
        Some(sample_format)
    );
    assert_eq!(
        sample_format.is_planar(),
        matches!(
            sample_format,
            SampleFormat::U8P
                | SampleFormat::S16P
                | SampleFormat::S32P
                | SampleFormat::FltP
                | SampleFormat::DblP
                | SampleFormat::S64P
        )
    );

    let Ok(plane_sizes) = sample_format.plane_sizes(samples_per_channel, channels) else {
        assert_eq!(channels, 0);
        assert!(AudioFrame::new(sample_rate, channels, sample_format, 1, vec![vec![0]]).is_err());
        return;
    };
    assert_eq!(
        sample_format.plane_count(channels).unwrap(),
        plane_sizes.len()
    );
    assert_eq!(
        sample_format.bytes_per_sample(),
        expected_sample_bytes(sample_format)
    );
    assert_eq!(
        sample_format.sample_bits(),
        sample_format.bytes_per_sample() * 8
    );
    assert_eq!(
        sample_format.family().bytes_per_sample(),
        sample_format.bytes_per_sample()
    );
    assert_eq!(
        sample_format.family().sample_bits(),
        sample_format.sample_bits()
    );
    assert_eq!(
        sample_format.family().numeric_kind(),
        sample_format.numeric_kind()
    );
    assert_eq!(sample_format.packed(), sample_format.with_planar(false));
    assert_eq!(sample_format.planar(), sample_format.with_planar(true));
    assert_eq!(sample_format.packed().planar(), sample_format.planar());
    assert_eq!(sample_format.planar().packed(), sample_format.packed());
    assert!(!sample_format.packed().is_planar());
    assert!(sample_format.planar().is_planar());
    assert_eq!(
        sample_format.is_float(),
        sample_format.numeric_kind() == SampleFormatNumericKind::Float
    );
    assert_eq!(
        sample_format.is_integer(),
        sample_format.numeric_kind() != SampleFormatNumericKind::Float
    );
    assert_eq!(
        sample_format.is_signed_integer(),
        sample_format.numeric_kind() == SampleFormatNumericKind::SignedInteger
    );
    assert_eq!(
        sample_format.is_unsigned_integer(),
        sample_format.numeric_kind() == SampleFormatNumericKind::UnsignedInteger
    );
    assert_eq!(
        sample_format.bytes_per_sample_frame(channels).unwrap(),
        usize::from(channels) * sample_format.bytes_per_sample()
    );

    if sample_rate == 0 {
        assert!(AudioFrame::new(sample_rate, channels, sample_format, 1, vec![vec![0]]).is_err());
        return;
    }

    let planes = plane_sizes
        .iter()
        .map(|size| payload_from(cursor, *size))
        .collect::<Vec<_>>();
    let audio = AudioFrame::new(
        sample_rate,
        channels,
        sample_format,
        samples_per_channel,
        planes.clone(),
    )
    .unwrap();
    assert_eq!(audio.sample_rate(), sample_rate);
    assert_eq!(audio.channels(), channels);
    assert_eq!(audio.sample_format(), sample_format);
    assert_eq!(audio.sample_format_name(), sample_format.name());
    assert_eq!(audio.samples_per_channel(), samples_per_channel);
    assert_eq!(audio.line_sizes(), plane_sizes.as_slice());
    assert_eq!(audio.planes(), planes.as_slice());
    assert_eq!(audio.plane_buffers().len(), planes.len());
    for (plane_buffer, plane) in audio.plane_buffers().iter().zip(&planes) {
        assert_eq!(plane_buffer.as_slice(), plane.as_slice());
    }
    let audio_plane_buffers = planes
        .iter()
        .map(|plane| BufferRef::copy_from_slice_with_padding(plane, 1).unwrap())
        .collect::<Vec<_>>();
    let audio_from_buffers = AudioFrame::new_with_buffer_refs(
        sample_rate,
        channels,
        sample_format,
        samples_per_channel,
        audio_plane_buffers.clone(),
    )
    .unwrap();
    assert_eq!(audio_from_buffers.planes(), planes.as_slice());
    for (stored, source) in audio_from_buffers
        .plane_buffers()
        .iter()
        .zip(&audio_plane_buffers)
    {
        assert!(stored.shares_storage(source));
        assert_eq!(stored.padding_slice(), &[0]);
    }

    let strided_audio_line_sizes = plane_sizes
        .iter()
        .map(|size| size.saturating_add(1))
        .collect::<Vec<_>>();
    let strided_audio_planes = planes
        .iter()
        .zip(&strided_audio_line_sizes)
        .map(|(plane, &line_size)| {
            let mut storage = plane.clone();
            storage.resize(line_size, 0xEE);
            storage
        })
        .collect::<Vec<_>>();
    let strided_audio = AudioFrame::new_with_line_sizes(
        sample_rate,
        channels,
        sample_format,
        samples_per_channel,
        strided_audio_planes.clone(),
        strided_audio_line_sizes.clone(),
    )
    .unwrap();
    assert_eq!(
        strided_audio.line_sizes(),
        strided_audio_line_sizes.as_slice()
    );
    assert_eq!(strided_audio.planes(), planes.as_slice());
    for (stored, strided) in strided_audio
        .plane_buffers()
        .iter()
        .zip(&strided_audio_planes)
    {
        assert_eq!(stored.as_slice(), strided.as_slice());
    }
    let audio_alignment = usize::from(cursor.next().unwrap_or_default() % 8) + 1;
    let aligned_audio_line_sizes = AudioFrame::aligned_line_sizes(
        sample_format,
        samples_per_channel,
        channels,
        audio_alignment,
    )
    .unwrap();
    for (line_size, visible_size) in aligned_audio_line_sizes.iter().zip(&plane_sizes) {
        assert!(*line_size >= *visible_size);
        assert_eq!(line_size % audio_alignment, 0);
    }
    let aligned_audio_planes = planes
        .iter()
        .zip(&aligned_audio_line_sizes)
        .map(|(plane, &line_size)| {
            let mut storage = plane.clone();
            storage.resize(line_size, 0xDD);
            storage
        })
        .collect::<Vec<_>>();
    let aligned_audio = AudioFrame::new_with_aligned_line_sizes(
        sample_rate,
        channels,
        sample_format,
        samples_per_channel,
        aligned_audio_planes.clone(),
        audio_alignment,
    )
    .unwrap();
    assert_eq!(
        aligned_audio.line_sizes(),
        aligned_audio_line_sizes.as_slice()
    );
    assert_eq!(aligned_audio.planes(), planes.as_slice());
    let aligned_audio_buffers = aligned_audio_planes
        .iter()
        .map(|plane| BufferRef::copy_from_slice(plane))
        .collect::<Vec<_>>();
    let aligned_audio_from_buffers = AudioFrame::new_with_buffer_refs_and_aligned_line_sizes(
        sample_rate,
        channels,
        sample_format,
        samples_per_channel,
        aligned_audio_buffers.clone(),
        audio_alignment,
    )
    .unwrap();
    assert_eq!(aligned_audio_from_buffers.planes(), planes.as_slice());
    for (stored, source) in aligned_audio_from_buffers
        .plane_buffers()
        .iter()
        .zip(&aligned_audio_buffers)
    {
        assert!(stored.shares_storage(source));
    }
    assert_eq!(
        AudioFrame::aligned_line_sizes(sample_format, samples_per_channel, channels, 0)
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidArgument
    );
    assert_eq!(
        AudioFrame::new_with_line_sizes(
            sample_rate,
            channels,
            sample_format,
            samples_per_channel,
            strided_audio_planes.clone(),
            Vec::new(),
        )
        .unwrap_err()
        .kind(),
        AvErrorKind::InvalidArgument
    );
    if plane_sizes[0] > 0 {
        let mut undersized_audio_line_sizes = strided_audio_line_sizes.clone();
        undersized_audio_line_sizes[0] = plane_sizes[0] - 1;
        assert_eq!(
            AudioFrame::new_with_line_sizes(
                sample_rate,
                channels,
                sample_format,
                samples_per_channel,
                strided_audio_planes.clone(),
                undersized_audio_line_sizes,
            )
            .unwrap_err()
            .kind(),
            AvErrorKind::InvalidArgument
        );
    }

    let mut mutable_strided_audio = strided_audio.clone();
    let shared_strided_audio = mutable_strided_audio.clone();
    assert!(!mutable_strided_audio.is_writable());
    let replacement_strided_audio_planes = planes
        .iter()
        .map(|plane| {
            plane
                .iter()
                .map(|byte| byte.wrapping_add(3))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for (index, replacement) in replacement_strided_audio_planes.iter().enumerate() {
        mutable_strided_audio
            .set_plane_visible_data(index, replacement)
            .unwrap();
    }
    assert!(mutable_strided_audio.is_writable());
    assert_eq!(
        mutable_strided_audio.planes(),
        replacement_strided_audio_planes.as_slice()
    );
    for (((stored, shared), original_strided), replacement) in mutable_strided_audio
        .plane_buffers()
        .iter()
        .zip(shared_strided_audio.plane_buffers())
        .zip(&strided_audio_planes)
        .zip(&replacement_strided_audio_planes)
    {
        assert!(!stored.shares_storage(shared));
        let mut expected_storage = original_strided.clone();
        expected_storage[..replacement.len()].copy_from_slice(replacement);
        assert_eq!(stored.as_slice(), expected_storage.as_slice());
    }

    let mut mutable_audio = audio_from_buffers.clone();
    let shared_audio = mutable_audio.clone();
    assert!(!mutable_audio.is_writable());
    let replacement_audio_planes = planes
        .iter()
        .map(|plane| {
            plane
                .iter()
                .map(|byte| byte.wrapping_add(1))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for (index, replacement) in replacement_audio_planes.iter().enumerate() {
        mutable_audio
            .set_plane_visible_data(index, replacement)
            .unwrap();
    }
    assert!(mutable_audio.is_writable());
    assert_eq!(mutable_audio.planes(), replacement_audio_planes.as_slice());
    for ((stored, shared), replacement) in mutable_audio
        .plane_buffers()
        .iter()
        .zip(shared_audio.plane_buffers())
        .zip(&replacement_audio_planes)
    {
        assert!(!stored.shares_storage(shared));
        assert_eq!(stored.as_slice(), replacement.as_slice());
    }
    assert_eq!(
        mutable_audio
            .set_plane_visible_data(replacement_audio_planes.len(), &[])
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidArgument
    );
    let wrong_audio = vec![0; replacement_audio_planes[0].len().saturating_add(1)];
    assert_eq!(
        mutable_audio
            .set_plane_visible_data(0, &wrong_audio)
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    assert_eq!(
        audio.channel_layout(),
        ChannelLayout::default_for_count(channels)
    );

    let mut frame = Frame::audio(audio);
    frame.set_pts(timestamp_from(cursor.next()));
    assert!(matches!(frame.data(), FrameData::Audio(_)));
    assert!(frame.is_writable());
    let shared_frame_payload = frame.clone();
    assert!(!frame.is_writable());
    frame.data_mut().make_writable();
    assert!(frame.data().is_writable());
    let frame_replacement = planes[0]
        .iter()
        .map(|byte| byte.wrapping_add(2))
        .collect::<Vec<_>>();
    frame
        .data_mut()
        .set_plane_visible_data(0, &frame_replacement)
        .unwrap();
    let (frame_audio, shared_audio) = match (frame.data(), shared_frame_payload.data()) {
        (FrameData::Audio(frame_audio), FrameData::Audio(shared_audio)) => {
            (frame_audio, shared_audio)
        }
        _ => unreachable!("constructed audio frame changed variant"),
    };
    assert_eq!(frame_audio.planes()[0], frame_replacement);
    assert_eq!(shared_audio.planes(), planes.as_slice());
    assert!(!frame_audio.plane_buffers()[0].shares_storage(&shared_audio.plane_buffers()[0]));

    let layout = channel_layout_from(cursor.next());
    if layout.channel_count() == channels {
        let frame = AudioFrame::new_with_channel_layout(
            sample_rate,
            layout,
            sample_format,
            samples_per_channel,
            planes.clone(),
        )
        .unwrap();
        assert_eq!(frame.channel_layout(), Some(layout));
        assert_eq!(frame.line_sizes(), plane_sizes.as_slice());
        let aligned_layout_frame = AudioFrame::new_with_channel_layout_and_aligned_line_sizes(
            sample_rate,
            layout,
            sample_format,
            samples_per_channel,
            aligned_audio_planes,
            audio_alignment,
        )
        .unwrap();
        assert_eq!(aligned_layout_frame.channel_layout(), Some(layout));
        assert_eq!(
            aligned_layout_frame.line_sizes(),
            aligned_audio_line_sizes.as_slice()
        );
        assert_eq!(aligned_layout_frame.planes(), planes.as_slice());
        let padded_frame = AudioFrame::new_with_channel_layout_and_line_sizes(
            sample_rate,
            layout,
            sample_format,
            samples_per_channel,
            strided_audio_planes,
            strided_audio_line_sizes,
        )
        .unwrap();
        assert_eq!(padded_frame.channel_layout(), Some(layout));
        assert_eq!(padded_frame.planes(), planes.as_slice());
    } else {
        assert_eq!(
            layout.validate_channel_count(channels).unwrap_err().kind(),
            AvErrorKind::InvalidArgument
        );
    }
}

fn exercise_packet_and_hashes(cursor: &mut Cursor<'_>) {
    let payload_len = usize::from(cursor.next().unwrap_or_default()) % (MAX_PAYLOAD + 1);
    let payload = payload_from(cursor, payload_len);
    let stream_index = usize::from(cursor.next().unwrap_or_default() % 4);
    let mut packet = Packet::new(payload.clone(), stream_index);
    assert_eq!(packet.data(), payload.as_slice());
    assert_eq!(packet.len(), payload.len());
    assert_eq!(packet.is_empty(), payload.is_empty());
    assert_eq!(packet.stream_index(), stream_index);
    assert_eq!(packet.pts(), None);
    assert_eq!(packet.dts(), None);
    assert_eq!(packet.duration(), 0);
    assert_eq!(packet.pos(), None);

    let pts = timestamp_from(cursor.next());
    let dts = timestamp_from(cursor.next());
    packet.set_pts(pts);
    packet.set_dts(dts);
    assert_eq!(packet.pts(), pts);
    assert_eq!(packet.dts(), dts);

    let duration = i64::from(cursor.next().unwrap_or_default());
    packet.set_duration(duration).unwrap();
    assert_eq!(packet.duration(), duration);
    assert_eq!(
        packet.set_duration(-1).unwrap_err().kind(),
        AvErrorKind::InvalidArgument
    );
    assert_eq!(packet.duration(), duration);

    let pos = if cursor.next().unwrap_or_default().is_multiple_of(3) {
        None
    } else {
        Some(i64::from(cursor.next().unwrap_or_default()))
    };
    packet.set_pos(pos).unwrap();
    assert_eq!(packet.pos(), pos);
    assert_eq!(
        packet.set_pos(Some(-1)).unwrap_err().kind(),
        AvErrorKind::InvalidArgument
    );
    assert_eq!(packet.pos(), pos);

    packet.set_key(cursor.next().unwrap_or_default().is_multiple_of(2));
    if packet.flags().contains(PacketFlags::KEY) {
        assert_ne!(packet.flags().bits() & PacketFlags::KEY.bits(), 0);
    }
    packet.set_flag(PacketFlags::KEY, false);
    assert!(!packet.flags().contains(PacketFlags::KEY));
    for flag in [
        PacketFlags::CORRUPT,
        PacketFlags::DISCARD,
        PacketFlags::TRUSTED,
        PacketFlags::DISPOSABLE,
    ] {
        packet.set_flag(flag, true);
        assert!(packet.flags().contains(flag));
        packet.set_flag(flag, false);
        assert!(!packet.flags().contains(flag));
    }
    let raw_flags = u32::from(cursor.next().unwrap_or_default()) | 0xffff_ff00;
    let truncated = PacketFlags::from_bits_truncate(raw_flags);
    assert_eq!(truncated.bits() & !PacketFlags::all().bits(), 0);

    let side_data_len = usize::from(cursor.next().unwrap_or_default() % 16);
    let side_data_payload = payload_from(cursor, side_data_len);
    let side_data = SideData::new("fuzz_side_data", side_data_payload.clone()).unwrap();
    packet.push_side_data(side_data);
    assert_eq!(packet.side_data()[0].kind(), "fuzz_side_data");
    assert_eq!(packet.side_data()[0].data(), side_data_payload.as_slice());
    assert_eq!(
        SideData::new(" \t", Vec::new()).unwrap_err().kind(),
        AvErrorKind::InvalidArgument
    );
    assert_eq!(
        SideData::new("bad\0kind", Vec::new()).unwrap_err().kind(),
        AvErrorKind::InvalidArgument
    );

    let split = usize::from(cursor.next().unwrap_or_default()) % (payload.len() + 1);
    let mut adler = Adler32::new();
    adler.update(&payload[..split]);
    adler.update(&payload[split..]);
    assert_eq!(adler.finalize(), adler32(&payload));

    let mut crc = Crc32::new();
    crc.update(&payload[..split]);
    crc.update(&payload[split..]);
    assert_eq!(crc.finalize(), crc32_ieee(&payload));

    let second_split =
        split + (usize::from(cursor.next().unwrap_or_default()) % (payload.len() - split + 1));
    let mut md5_state = Md5::new();
    md5_state.update(&payload[..split]);
    md5_state.update(&payload[split..second_split]);
    md5_state.update(&payload[second_split..]);
    let md5_digest = md5_state.finalize();
    assert_eq!(md5_digest, md5(&payload));
    assert_eq!(digest_to_hex(&md5_digest).len(), 32);

    let third_split = second_split
        + (usize::from(cursor.next().unwrap_or_default()) % (payload.len() - second_split + 1));
    let mut sha256_state = Sha256::new();
    sha256_state.update(&payload[..split]);
    sha256_state.update(&payload[split..second_split]);
    sha256_state.update(&payload[second_split..third_split]);
    sha256_state.update(&payload[third_split..]);
    let sha256_digest = sha256_state.finalize();
    assert_eq!(sha256_digest, sha256(&payload));
    assert_eq!(digest_to_hex(&sha256_digest).len(), 64);

    let mut sha224_state = Sha224::new();
    sha224_state.update(&payload[..split]);
    sha224_state.update(&payload[split..second_split]);
    sha224_state.update(&payload[second_split..third_split]);
    sha224_state.update(&payload[third_split..]);
    let sha224_digest = sha224_state.finalize();
    assert_eq!(sha224_digest, sha224(&payload));
    assert_eq!(digest_to_hex(&sha224_digest).len(), 56);

    let mut sha384_state = Sha384::new();
    sha384_state.update(&payload[..split]);
    sha384_state.update(&payload[split..second_split]);
    sha384_state.update(&payload[second_split..third_split]);
    sha384_state.update(&payload[third_split..]);
    let sha384_digest = sha384_state.finalize();
    assert_eq!(sha384_digest, sha384(&payload));
    assert_eq!(digest_to_hex(&sha384_digest).len(), 96);

    let mut sha512_state = Sha512::new();
    sha512_state.update(&payload[..split]);
    sha512_state.update(&payload[split..second_split]);
    sha512_state.update(&payload[second_split..third_split]);
    sha512_state.update(&payload[third_split..]);
    let sha512_digest = sha512_state.finalize();
    assert_eq!(sha512_digest, sha512(&payload));
    assert_eq!(digest_to_hex(&sha512_digest).len(), 128);
}

fn exercise_fixtures() {
    assert_eq!(PixelFormat::from_name("gray8"), Some(PixelFormat::Gray8));
    assert_eq!(
        PixelFormat::Yuv420p.plane_sizes(2, 2).unwrap(),
        vec![4, 1, 1]
    );
    assert_eq!(SampleFormat::U8.plane_sizes(2, 2).unwrap(), vec![4]);
    assert_eq!(SampleFormat::S16.plane_sizes(2, 2).unwrap(), vec![8]);
    assert_eq!(SampleFormat::S32.plane_sizes(2, 2).unwrap(), vec![16]);
    assert_eq!(SampleFormat::Flt.plane_sizes(2, 2).unwrap(), vec![16]);
    assert_eq!(SampleFormat::Dbl.plane_sizes(2, 2).unwrap(), vec![32]);
    assert_eq!(SampleFormat::U8P.plane_sizes(2, 2).unwrap(), vec![2, 2]);
    assert_eq!(SampleFormat::S16P.plane_sizes(2, 2).unwrap(), vec![4, 4]);
    assert_eq!(SampleFormat::S32P.plane_sizes(2, 2).unwrap(), vec![8, 8]);
    assert_eq!(SampleFormat::FltP.plane_sizes(2, 2).unwrap(), vec![8, 8]);
    assert_eq!(SampleFormat::DblP.plane_sizes(2, 2).unwrap(), vec![16, 16]);
    assert_eq!(SampleFormat::S64.plane_sizes(2, 2).unwrap(), vec![32]);
    assert_eq!(SampleFormat::S64P.plane_sizes(2, 2).unwrap(), vec![16, 16]);
    assert!(ChannelLayout::stereo().contains(Channel::FrontLeft));
    assert!(!ChannelLayout::stereo().contains(Channel::LowFrequency));
    assert_eq!(
        digest_to_hex(&md5(b"abc")),
        "900150983cd24fb0d6963f7d28e17f72"
    );
    assert_eq!(
        digest_to_hex(&sha256(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        digest_to_hex(&sha224(b"abc")),
        "23097d223405d8228642a477bda255b32aadbce4bda0b3f7e36c9da7"
    );
    assert_eq!(
        digest_to_hex(&sha384(b"abc")),
        "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7"
    );
    assert_eq!(
        digest_to_hex(&sha512(b"abc")),
        "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
    );

    let video = VideoFrame::new(1, 1, PixelFormat::Rgb24, vec![vec![1, 2, 3]]).unwrap();
    assert_eq!(video.line_sizes(), &[3]);
    assert_eq!(Frame::video(video).pts(), None);
    let mut empty_frame = Frame::empty();
    assert!(empty_frame.is_empty());
    assert!(!empty_frame.is_writable());
    assert!(matches!(empty_frame.data(), FrameData::Empty));
    assert_eq!(
        empty_frame
            .set_plane_visible_data(0, &[])
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidArgument
    );
    empty_frame.unref();
    assert!(empty_frame.is_empty());

    let ref_video = VideoFrame::new(1, 1, PixelFormat::Gray8, vec![vec![7]]).unwrap();
    let ref_side = BufferRef::copy_from_slice(&[0x44]);
    let ref_hw = BufferRef::copy_from_slice(&[0x55]);
    let mut source_frame = Frame::video(ref_video).with_hw_frames_context(ref_hw.clone());
    source_frame
        .set_side_data_kind_buffer(FrameSideDataKind::DisplayMatrix, ref_side.clone())
        .unwrap();
    let mut referenced_frame = Frame::empty();
    referenced_frame.ref_from(&source_frame);
    assert!(!referenced_frame.is_empty());
    assert!(referenced_frame.side_data()[0]
        .buffer()
        .shares_storage(&ref_side));
    assert!(referenced_frame
        .hw_frames_context()
        .unwrap()
        .shares_storage(&ref_hw));
    let mut moved_frame = Frame::empty();
    moved_frame.move_ref_from(&mut referenced_frame);
    assert!(referenced_frame.is_empty());
    assert!(!moved_frame.is_empty());
    assert!(moved_frame.side_data()[0]
        .buffer()
        .shares_storage(&ref_side));
    moved_frame.unref();
    assert!(moved_frame.is_empty());

    let mut permission_frame = Frame::video(
        VideoFrame::new_with_buffer_refs(
            1,
            1,
            PixelFormat::Gray8,
            vec![BufferRef::from_vec_readonly(vec![1])],
        )
        .unwrap(),
    )
    .with_hw_frames_context(BufferRef::from_vec_readonly(vec![0x20]));
    permission_frame
        .set_side_data_kind_buffer(
            FrameSideDataKind::IccProfile,
            BufferRef::from_vec_readonly(vec![0x10]),
        )
        .unwrap();
    let permission_clone = permission_frame.clone();
    assert!(!permission_frame.is_writable());
    assert!(!permission_frame.side_data_is_writable());
    assert_eq!(
        permission_frame.hw_frames_context_is_writable(),
        Some(false)
    );
    assert!(!permission_frame.all_references_are_writable());
    permission_frame.make_all_references_writable();
    assert!(permission_frame.all_references_are_writable());
    assert!(!permission_frame.side_data()[0]
        .buffer()
        .shares_storage(permission_clone.side_data()[0].buffer()));
    assert!(!permission_frame
        .hw_frames_context()
        .unwrap()
        .shares_storage(permission_clone.hw_frames_context().unwrap()));
    permission_frame
        .side_data_by_kind_mut(&FrameSideDataKind::IccProfile)
        .unwrap()
        .data_mut()[0] ^= 0xFF;
    assert_ne!(
        permission_frame.side_data()[0].data(),
        permission_clone.side_data()[0].data()
    );

    let mut properties_frame =
        Frame::video(VideoFrame::new(1, 1, PixelFormat::Gray8, vec![vec![1]]).unwrap());
    properties_frame
        .add_side_data_kind(FrameSideDataKind::DisplayMatrix, vec![1])
        .unwrap();
    properties_frame
        .add_side_data_kind(FrameSideDataKind::MotionVectors, vec![2])
        .unwrap();
    properties_frame
        .add_side_data_kind(FrameSideDataKind::SeiUnregistered, vec![3])
        .unwrap();
    properties_frame
        .add_side_data("vendor.private.side-data", vec![4])
        .unwrap();
    let removed_global =
        properties_frame.remove_side_data_by_properties(FrameSideDataProperties::GLOBAL);
    assert_eq!(removed_global.len(), 1);
    assert_eq!(
        removed_global[0].kind_id(),
        &FrameSideDataKind::DisplayMatrix
    );
    let removed_size_or_multi = properties_frame.remove_side_data_by_properties(
        FrameSideDataProperties::SIZE_DEPENDENT.union(FrameSideDataProperties::MULTI),
    );
    assert_eq!(removed_size_or_multi.len(), 2);
    assert_eq!(properties_frame.side_data().len(), 1);
    assert_eq!(
        properties_frame.side_data()[0].kind(),
        "vendor.private.side-data"
    );

    let afd = FrameSideData::new_active_format_description(
        FrameActiveFormatDescription::SixteenNineProtectedFourteenNine,
    )
    .unwrap();
    assert_eq!(afd.data(), &[14]);
    assert_eq!(
        afd.active_format_description().unwrap(),
        Some(FrameActiveFormatDescription::SixteenNineProtectedFourteenNine)
    );
    assert_eq!(
        FrameActiveFormatDescription::Same.ffmpeg_constant(),
        "AV_AFD_SAME"
    );
    assert_eq!(
        FrameActiveFormatDescription::from_byte(12)
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    assert_eq!(
        FrameSideData::new_with_kind(FrameSideDataKind::ActiveFormatDescription, vec![8, 9])
            .unwrap()
            .active_format_description()
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_afd = FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![8]).unwrap();
    assert_eq!(non_afd.active_format_description().unwrap(), None);

    let skip_samples = FrameSkipSamples::new(
        1024,
        256,
        FrameSkipSamplesReason::PaddingSilence,
        FrameSkipSamplesReason::Convergence,
    );
    let skip_side_data = FrameSideData::new_skip_samples(skip_samples).unwrap();
    assert_eq!(skip_side_data.skip_samples().unwrap(), Some(skip_samples));
    assert_eq!(skip_side_data.data(), &[0, 4, 0, 0, 0, 1, 0, 0, 0, 1]);
    assert_eq!(
        FrameSkipSamplesReason::from_byte(2).unwrap_err().kind(),
        AvErrorKind::InvalidData
    );
    assert_eq!(
        FrameSideData::new_with_kind(FrameSideDataKind::SkipSamples, vec![0; 9])
            .unwrap()
            .skip_samples()
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_skip =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0; 10]).unwrap();
    assert_eq!(non_skip.skip_samples().unwrap(), None);

    let display_matrix =
        FrameDisplayMatrix::new([1 << 16, 0, 0, 0, 1 << 16, 0, 12 << 16, -34 << 16, 1 << 30]);
    let display_matrix_side_data = FrameSideData::new_display_matrix(display_matrix).unwrap();
    assert_eq!(
        display_matrix_side_data.kind_id(),
        &FrameSideDataKind::DisplayMatrix
    );
    assert_eq!(
        display_matrix_side_data.display_matrix().unwrap(),
        Some(display_matrix)
    );
    assert_eq!(
        FrameDisplayMatrix::parse(&display_matrix.to_bytes()).unwrap(),
        display_matrix
    );
    assert_eq!(
        FrameDisplayMatrix::identity().elements(),
        [1 << 16, 0, 0, 0, 1 << 16, 0, 0, 0, 1 << 30]
    );
    assert_eq!(
        FrameDisplayMatrix::parse(&[0; FrameDisplayMatrix::DATA_LEN - 1])
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_display =
        FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0; 36]).unwrap();
    assert_eq!(non_display.display_matrix().unwrap(), None);

    let matrix_encoding =
        FrameSideData::new_matrix_encoding(FrameMatrixEncoding::DolbyProLogicIiX).unwrap();
    assert_eq!(
        matrix_encoding.matrix_encoding().unwrap(),
        Some(FrameMatrixEncoding::DolbyProLogicIiX)
    );
    assert_eq!(
        matrix_encoding.data(),
        &FrameMatrixEncoding::DolbyProLogicIiX.as_raw().to_ne_bytes()
    );
    assert_eq!(
        FrameMatrixEncoding::DolbyHeadphone.ffmpeg_constant(),
        "AV_MATRIX_ENCODING_DOLBYHEADPHONE"
    );
    assert_eq!(
        FrameMatrixEncoding::from_raw(7).unwrap_err().kind(),
        AvErrorKind::InvalidData
    );
    assert_eq!(
        FrameSideData::new_with_kind(FrameSideDataKind::MatrixEncoding, vec![0; 5])
            .unwrap()
            .matrix_encoding()
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_matrix =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0; 4]).unwrap();
    assert_eq!(non_matrix.matrix_encoding().unwrap(), None);

    let downmix_info = FrameDownmixInfo::new(
        FrameDownmixType::LtRt,
        1.0,
        0.7071067811865476,
        0.5,
        0.25,
        0.0,
    );
    let downmix_side_data = FrameSideData::new_downmix_info(downmix_info).unwrap();
    assert_eq!(downmix_side_data.kind_id(), &FrameSideDataKind::DownmixInfo);
    assert_eq!(
        downmix_side_data.downmix_info().unwrap(),
        Some(downmix_info)
    );
    assert_eq!(&downmix_side_data.data()[4..8], &[0, 0, 0, 0]);
    assert_eq!(
        FrameDownmixType::DolbyProLogicIi.ffmpeg_constant(),
        "AV_DOWNMIX_TYPE_DPLII"
    );
    assert_eq!(
        FrameDownmixType::from_raw(4).unwrap_err().kind(),
        AvErrorKind::InvalidData
    );
    let mut downmix_with_padding = downmix_info.to_bytes();
    downmix_with_padding[4..8].copy_from_slice(&[1, 2, 3, 4]);
    assert_eq!(
        FrameDownmixInfo::parse(&downmix_with_padding).unwrap(),
        downmix_info
    );
    let downmix_with_nan =
        FrameDownmixInfo::from_level_bits(FrameDownmixType::LoRo, [f64::NAN.to_bits(), 1, 2, 3, 4]);
    assert_eq!(downmix_with_nan.level_bits()[0], f64::NAN.to_bits());
    assert!(downmix_with_nan.center_mix_level().is_nan());
    assert_eq!(
        FrameSideData::new_with_kind(FrameSideDataKind::DownmixInfo, vec![0; 47])
            .unwrap()
            .downmix_info()
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_downmix =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0; 48]).unwrap();
    assert_eq!(non_downmix.downmix_info().unwrap(), None);

    let audio_service =
        FrameSideData::new_audio_service_type(FrameAudioServiceType::VoiceOver).unwrap();
    assert_eq!(
        audio_service.audio_service_type().unwrap(),
        Some(FrameAudioServiceType::VoiceOver)
    );
    assert_eq!(
        audio_service.data(),
        &FrameAudioServiceType::VoiceOver.as_raw().to_ne_bytes()
    );
    assert_eq!(
        FrameAudioServiceType::Karaoke.ffmpeg_constant(),
        "AV_AUDIO_SERVICE_TYPE_KARAOKE"
    );
    assert_eq!(
        FrameAudioServiceType::from_raw(9).unwrap_err().kind(),
        AvErrorKind::InvalidData
    );
    assert_eq!(
        FrameSideData::new_with_kind(FrameSideDataKind::AudioServiceType, vec![0; 3])
            .unwrap()
            .audio_service_type()
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_audio_service =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0; 4]).unwrap();
    assert_eq!(non_audio_service.audio_service_type().unwrap(), None);

    let gop_timecode = FrameGopTimecode::new(0x01FE_DCBA).unwrap();
    let gop_side_data = FrameSideData::new_gop_timecode(gop_timecode).unwrap();
    assert_eq!(gop_side_data.kind_id(), &FrameSideDataKind::GopTimecode);
    assert_eq!(gop_side_data.gop_timecode().unwrap(), Some(gop_timecode));
    assert_eq!(gop_timecode.as_raw_i64(), 0x01FE_DCBA);
    assert_eq!(
        FrameGopTimecode::parse(&gop_timecode.to_bytes()).unwrap(),
        gop_timecode
    );
    assert_eq!(
        FrameGopTimecode::from_raw_i64(FrameGopTimecode::MAX_VALUE)
            .unwrap()
            .as_raw_i64(),
        FrameGopTimecode::MAX_VALUE
    );
    assert_eq!(
        FrameGopTimecode::from_raw_i64(-1).unwrap_err().kind(),
        AvErrorKind::InvalidData
    );
    assert_eq!(
        FrameGopTimecode::from_raw_i64(FrameGopTimecode::MAX_VALUE + 1)
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    assert_eq!(
        FrameGopTimecode::parse(&[0; FrameGopTimecode::DATA_LEN - 1])
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_gop =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0; 8]).unwrap();
    assert_eq!(non_gop.gop_timecode().unwrap(), None);

    let s12m_timecode = FrameS12mTimecode::new(&[0x0102_0304, 0xA0B0_C0D0]).unwrap();
    let s12m_side_data = FrameSideData::new_s12m_timecode(s12m_timecode).unwrap();
    assert_eq!(s12m_side_data.kind_id(), &FrameSideDataKind::S12mTimecode);
    assert_eq!(s12m_side_data.s12m_timecode().unwrap(), Some(s12m_timecode));
    assert_eq!(s12m_timecode.count(), 2);
    assert_eq!(s12m_timecode.timecodes(), &[0x0102_0304, 0xA0B0_C0D0]);
    assert_eq!(s12m_timecode.raw_words(), [2, 0x0102_0304, 0xA0B0_C0D0, 0]);
    let s12m_with_unused =
        FrameS12mTimecode::from_raw_words([1, 0x0A0B_0C0D, 0xFEED_C0DE, 0x1234_5678]).unwrap();
    assert_eq!(s12m_with_unused.timecodes(), &[0x0A0B_0C0D]);
    assert_eq!(
        FrameS12mTimecode::parse(&s12m_with_unused.to_bytes()).unwrap(),
        s12m_with_unused
    );
    assert_eq!(
        FrameS12mTimecode::new(&[1, 2, 3, 4]).unwrap_err().kind(),
        AvErrorKind::InvalidArgument
    );
    assert_eq!(
        FrameS12mTimecode::parse(&[0; FrameS12mTimecode::DATA_LEN - 1])
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    assert_eq!(
        FrameS12mTimecode::from_raw_words([0, 1, 2, 3])
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_s12m =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0; 16]).unwrap();
    assert_eq!(non_s12m.s12m_timecode().unwrap(), None);

    let sei_uuid = [0xA5; FrameSeiUnregistered::UUID_LEN];
    let sei_payload =
        FrameSideData::new_sei_unregistered(sei_uuid, vec![0x01, 0x02, 0x03]).unwrap();
    let parsed_sei = sei_payload.sei_unregistered().unwrap().unwrap();
    assert_eq!(parsed_sei.uuid(), sei_uuid);
    assert_eq!(parsed_sei.user_data(), &[0x01, 0x02, 0x03]);
    assert_eq!(
        FrameSeiUnregistered::parse(&[0; FrameSeiUnregistered::UUID_LEN - 1])
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_sei = FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0]).unwrap();
    assert_eq!(non_sei.sei_unregistered().unwrap(), None);

    let audio = AudioFrame::new_with_channel_layout(
        48_000,
        ChannelLayout::stereo(),
        SampleFormat::S16,
        1,
        vec![vec![0, 0, 1, 0]],
    )
    .unwrap();
    assert_eq!(audio.channel_layout(), Some(ChannelLayout::stereo()));
    assert_eq!(audio.line_sizes(), &[4]);

    let planar_audio = AudioFrame::new_with_channel_layout(
        48_000,
        ChannelLayout::stereo(),
        SampleFormat::S16P,
        1,
        vec![vec![0, 0], vec![1, 0]],
    )
    .unwrap();
    assert_eq!(planar_audio.sample_format_name(), "s16p");
    assert_eq!(planar_audio.planes(), &[vec![0, 0], vec![1, 0]]);
    assert_eq!(planar_audio.line_sizes(), &[2, 2]);

    let planar_float_audio = AudioFrame::new_with_channel_layout(
        48_000,
        ChannelLayout::stereo(),
        SampleFormat::FltP,
        1,
        vec![vec![0; 4], vec![1; 4]],
    )
    .unwrap();
    assert_eq!(planar_float_audio.sample_format_name(), "fltp");
    assert_eq!(planar_float_audio.planes(), &[vec![0; 4], vec![1; 4]]);
    assert_eq!(planar_float_audio.line_sizes(), &[4, 4]);
}

fn expected_rescale(
    value: i64,
    src: Rational,
    dst: Rational,
    rounding: Rounding,
) -> Result<i64, ()> {
    let numerator = i128::from(value) * i128::from(src.num()) * i128::from(dst.den());
    let denominator = i128::from(src.den()) * i128::from(dst.num());
    i64::try_from(div_round(numerator, denominator, rounding)).map_err(|_| ())
}

fn div_round(numerator: i128, denominator: i128, rounding: Rounding) -> i128 {
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    if remainder == 0 {
        return quotient;
    }

    let same_sign = (numerator >= 0) == (denominator >= 0);
    match rounding {
        Rounding::Zero => quotient,
        Rounding::Inf => {
            if same_sign {
                quotient + 1
            } else {
                quotient - 1
            }
        }
        Rounding::Down => {
            if same_sign {
                quotient
            } else {
                quotient - 1
            }
        }
        Rounding::Up => {
            if same_sign {
                quotient + 1
            } else {
                quotient
            }
        }
        Rounding::NearInf => {
            if remainder.abs() * 2 >= denominator.abs() {
                if same_sign {
                    quotient + 1
                } else {
                    quotient - 1
                }
            } else {
                quotient
            }
        }
    }
}

fn pixel_format_from(byte: Option<u8>) -> PixelFormat {
    match byte.unwrap_or_default() % 4 {
        0 => PixelFormat::Gray8,
        1 => PixelFormat::Rgb24,
        2 => PixelFormat::Rgba,
        _ => PixelFormat::Yuv420p,
    }
}

fn sample_format_from(byte: Option<u8>) -> SampleFormat {
    let formats = SampleFormat::ALL;
    formats[usize::from(byte.unwrap_or_default()) % formats.len()]
}

fn expected_sample_bytes(format: SampleFormat) -> usize {
    match format {
        SampleFormat::U8 | SampleFormat::U8P => 1,
        SampleFormat::S16 | SampleFormat::S16P => 2,
        SampleFormat::S32 | SampleFormat::S32P | SampleFormat::Flt | SampleFormat::FltP => 4,
        SampleFormat::Dbl | SampleFormat::DblP | SampleFormat::S64 | SampleFormat::S64P => 8,
    }
}

fn channel_layout_from(byte: Option<u8>) -> ChannelLayout {
    match byte.unwrap_or_default() % 6 {
        0 => ChannelLayout::mono(),
        1 => ChannelLayout::stereo(),
        2 => ChannelLayout::quad(),
        3 => ChannelLayout::five_one(),
        4 => ChannelLayout::five_one_side(),
        _ => ChannelLayout::seven_one(),
    }
}

fn frame_side_data_kind_from(byte: Option<u8>) -> FrameSideDataKind {
    match byte.unwrap_or_default() % 20 {
        0 => FrameSideDataKind::DisplayMatrix,
        1 => FrameSideDataKind::MatrixEncoding,
        2 => FrameSideDataKind::DownmixInfo,
        3 => FrameSideDataKind::ReplayGain,
        4 => FrameSideDataKind::MasteringDisplayMetadata,
        5 => FrameSideDataKind::ContentLightLevel,
        6 => FrameSideDataKind::IccProfile,
        7 => FrameSideDataKind::DolbyVisionRpuBuffer,
        8 => FrameSideDataKind::Lcevc,
        9 => FrameSideDataKind::GopTimecode,
        10 => FrameSideDataKind::S12mTimecode,
        11 => FrameSideDataKind::VideoHint,
        12 => FrameSideDataKind::ViewId,
        13 => FrameSideDataKind::ThreeDReferenceDisplays,
        14 => FrameSideDataKind::Exif,
        15 => FrameSideDataKind::SeiUnregistered,
        16 => FrameSideDataKind::ActiveFormatDescription,
        17 => FrameSideDataKind::SkipSamples,
        18 => FrameSideDataKind::AudioServiceType,
        _ => FrameSideDataKind::Unknown(String::from("fuzz_frame_side_data")),
    }
}

fn expected_video_line_sizes(pixel_format: PixelFormat, width: usize) -> Vec<usize> {
    match pixel_format {
        PixelFormat::Gray8 => vec![width],
        PixelFormat::Rgb24 => vec![width * 3],
        PixelFormat::Rgba => vec![width * 4],
        PixelFormat::Yuv420p => vec![width, width / 2, width / 2],
    }
}

fn expected_video_plane_shapes(
    pixel_format: PixelFormat,
    width: usize,
    height: usize,
) -> Vec<(usize, usize)> {
    match pixel_format {
        PixelFormat::Gray8 => vec![(width, height)],
        PixelFormat::Rgb24 => vec![(width * 3, height)],
        PixelFormat::Rgba => vec![(width * 4, height)],
        PixelFormat::Yuv420p => vec![
            (width, height),
            (width / 2, height / 2),
            (width / 2, height / 2),
        ],
    }
}

fn dimension_from(byte: Option<u8>) -> usize {
    usize::from(byte.unwrap_or_default() % 9)
}

fn sample_rate_from(byte: Option<u8>) -> u32 {
    match byte.unwrap_or_default() % 5 {
        0 => 0,
        1 => 8_000,
        2 => 44_100,
        3 => 48_000,
        _ => 96_000,
    }
}

fn channel_count_from(byte: Option<u8>) -> u16 {
    match byte.unwrap_or_default() % 7 {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 4,
        4 => 6,
        5 => 8,
        _ => u16::from(byte.unwrap_or_default() % 12),
    }
}

fn timestamp_from(byte: Option<u8>) -> Option<i64> {
    match byte.unwrap_or_default() % 4 {
        0 => None,
        1 => Some(0),
        2 => Some(i64::from(byte.unwrap_or_default())),
        _ => Some(-i64::from(byte.unwrap_or_default())),
    }
}

fn positive_rational_from(num: Option<u8>, den: Option<u8>) -> Rational {
    Rational::new(
        i32::from(num.unwrap_or_default() % 31) + 1,
        i32::from(den.unwrap_or_default() % 31) + 1,
    )
    .unwrap()
}

fn small_i32_from(byte: Option<u8>) -> i32 {
    i32::from(byte.unwrap_or_default()) - 128
}

fn small_i64_from(first: Option<u8>, second: Option<u8>) -> i64 {
    let high = i64::from(first.unwrap_or_default()) - 128;
    let low = i64::from(second.unwrap_or_default());
    high * 256 + low
}

fn io_error_kind_from(byte: Option<u8>) -> io::ErrorKind {
    match byte.unwrap_or_default() % 6 {
        0 => io::ErrorKind::NotFound,
        1 => io::ErrorKind::UnexpectedEof,
        2 => io::ErrorKind::InvalidData,
        3 => io::ErrorKind::InvalidInput,
        4 => io::ErrorKind::Unsupported,
        _ => io::ErrorKind::PermissionDenied,
    }
}

fn expected_av_error_kind_for_io(kind: io::ErrorKind) -> AvErrorKind {
    match kind {
        io::ErrorKind::NotFound => AvErrorKind::NotFound,
        io::ErrorKind::UnexpectedEof => AvErrorKind::EndOfFile,
        io::ErrorKind::InvalidData => AvErrorKind::InvalidData,
        io::ErrorKind::InvalidInput => AvErrorKind::InvalidArgument,
        io::ErrorKind::Unsupported => AvErrorKind::Unsupported,
        _ => AvErrorKind::External,
    }
}

fn payload_from(cursor: &mut Cursor<'_>, len: usize) -> Vec<u8> {
    let mut payload = Vec::with_capacity(len);
    for _ in 0..len {
        payload.push(cursor.next().unwrap_or_default());
    }
    payload
}

struct Cursor<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }

    fn next(&mut self) -> Option<u8> {
        let byte = self.data.get(self.offset).copied();
        self.offset = self.offset.saturating_add(usize::from(byte.is_some()));
        byte
    }
}
