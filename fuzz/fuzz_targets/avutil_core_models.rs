#![no_main]

use avutil::{
    adler32, crc32_ieee, digest_to_hex, md5, rescale_q, rescale_q_rnd, rescale_q_rnd_pass_minmax,
    sha224, sha256, sha384, sha512, Adler32, AudioFrame, AvError, AvErrorKind, BufferPool,
    BufferPoolCallbacks, BufferRef, Channel, ChannelLayout, Crc32, Frame, FrameA53ClosedCaptions,
    FrameActiveFormatDescription, FrameAmbientViewingEnvironment, FrameAudioServiceType,
    FrameContentLightMetadata, FrameData, FrameDetectionBbox, FrameDetectionBboxes,
    FrameDisplayMatrix, FrameDolbyVisionColorMetadata, FrameDolbyVisionDataMapping,
    FrameDolbyVisionDmData, FrameDolbyVisionMetadata, FrameDolbyVisionRpuBuffer,
    FrameDolbyVisionRpuDataHeader, FrameDownmixInfo, FrameDownmixType, FrameDynamicHdrPlus,
    FrameDynamicHdrVivid, FrameExif, FrameExifEndian, FrameExifEntry, FrameExifExposureProgram,
    FrameExifFlash, FrameExifGpsAltitudeRef, FrameExifGpsDirectionRef, FrameExifGpsDistanceRef,
    FrameExifGpsLatitudeRef, FrameExifGpsLongitudeRef, FrameExifGpsMeasureMode,
    FrameExifGpsSpeedRef, FrameExifGpsStatus, FrameExifIfdPointerKind, FrameExifLightSource,
    FrameExifMeteringMode, FrameExifOrientation, FrameExifResolutionUnit, FrameExifTiffType,
    FrameExifWhiteBalance, FrameFilmGrainAomParams, FrameFilmGrainH274Params, FrameFilmGrainParams,
    FrameFilmGrainParamsType, FrameGopTimecode, FrameHdrPlusColorTransformParams,
    FrameHdrPlusOverlapProcessOption, FrameHdrVivid3SplineParams,
    FrameHdrVividColorToneMappingParams, FrameHdrVividColorTransformParams, FrameIccProfile,
    FrameLcevc, FrameMasteringDisplayMetadata, FrameMatrixEncoding, FrameMotionVector,
    FrameMotionVectors, FramePanScan, FrameRegionOfInterest, FrameRegionsOfInterest,
    FrameReplayGain, FrameS12mTimecode, FrameSeiUnregistered, FrameSideData, FrameSideDataKind,
    FrameSideDataProperties, FrameSkipSamples, FrameSkipSamplesReason, FrameSphericalMapping,
    FrameSphericalProjection, FrameStereo3d, FrameStereo3dFlags, FrameStereo3dPrimaryEye,
    FrameStereo3dType, FrameStereo3dView, FrameThreeDReferenceDisplay,
    FrameThreeDReferenceDisplays, FrameVideoBlockParams, FrameVideoEncParams,
    FrameVideoEncParamsType, FrameVideoHint, FrameVideoHintType, FrameVideoRect, FrameViewId, Md5,
    Packet, PacketFlags, PixelFormat, Rational, Rounding, SampleFormat, SampleFormatNumericKind,
    Sha224, Sha256, Sha384, Sha512, SideData, VideoFrame,
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
    match frame.side_data()[0].pan_scan() {
        Ok(Some(value)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::PanScan);
            assert_eq!(frame_side_data_payload.len(), FramePanScan::DATA_LEN);
            assert_eq!(value.to_bytes().as_slice(), frame_side_data_payload);
            assert_eq!(value.position().len(), FramePanScan::POSITIONS);
            for index in 0..FramePanScan::POSITIONS {
                assert!(value.field_position(index).is_some());
            }
            assert!(value.field_position(FramePanScan::POSITIONS).is_none());
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::PanScan),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::PanScan);
            assert!(pan_scan_payload_invalid(&frame_side_data_payload));
        }
    }
    match frame.side_data()[0].a53_closed_captions() {
        Ok(Some(value)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::A53ClosedCaptions);
            assert_eq!(value.data(), frame_side_data_payload.as_slice());
            assert_eq!(
                frame_side_data_payload.len(),
                value.entry_count() * FrameA53ClosedCaptions::BYTES_PER_CC
            );
            assert_eq!(value.is_empty(), frame_side_data_payload.is_empty());
            assert_eq!(value.entries().count(), value.entry_count());
            for index in 0..value.entry_count() {
                assert!(value.entry(index).is_some());
            }
            assert!(value.entry(value.entry_count()).is_none());
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::A53ClosedCaptions),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::A53ClosedCaptions);
            assert!(a53_closed_captions_payload_invalid(
                &frame_side_data_payload
            ));
        }
    }
    match frame.side_data()[0].stereo3d() {
        Ok(Some(value)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::Stereo3d);
            assert_eq!(frame_side_data_payload.len(), FrameStereo3d::DATA_LEN);
            assert_eq!(value.to_bytes().as_slice(), frame_side_data_payload);
            assert_eq!(
                FrameStereo3d::parse(value.to_bytes().as_slice()).unwrap(),
                value
            );
            assert_eq!(
                FrameStereo3dType::from_raw(value.stereo_type().as_raw()).unwrap(),
                value.stereo_type()
            );
            assert_eq!(
                FrameStereo3dFlags::from_raw(value.flags().as_raw()).unwrap(),
                value.flags()
            );
            assert_eq!(
                FrameStereo3dView::from_raw(value.view().as_raw()).unwrap(),
                value.view()
            );
            assert_eq!(
                FrameStereo3dPrimaryEye::from_raw(value.primary_eye().as_raw()).unwrap(),
                value.primary_eye()
            );
            assert_eq!(
                value.has_inverted_views(),
                value.flags().contains(FrameStereo3dFlags::INVERT)
            );
            assert_eq!(value.flags().bits() & !FrameStereo3dFlags::ALL.bits(), 0);
            assert!(stereo3d_disparity_valid(
                value.horizontal_disparity_adjustment()
            ));
            assert!(stereo3d_field_of_view_valid(
                value.horizontal_field_of_view()
            ));
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::Stereo3d),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::Stereo3d);
            assert!(stereo3d_payload_invalid(&frame_side_data_payload));
        }
    }
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
    match frame.side_data()[0].replay_gain() {
        Ok(Some(value)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::ReplayGain);
            assert_eq!(frame_side_data_payload.len(), FrameReplayGain::DATA_LEN);
            assert_eq!(value.to_bytes().as_slice(), frame_side_data_payload);
            assert_eq!(
                FrameReplayGain::new(
                    value.track_gain(),
                    value.track_peak(),
                    value.album_gain(),
                    value.album_peak()
                ),
                value
            );
            assert_eq!(FrameReplayGain::parse(&value.to_bytes()).unwrap(), value);
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::ReplayGain),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::ReplayGain);
            assert_ne!(frame_side_data_payload.len(), FrameReplayGain::DATA_LEN);
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
    match frame.side_data()[0].motion_vectors() {
        Ok(Some(payload)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::MotionVectors);
            assert!(!frame_side_data_payload.is_empty());
            assert_eq!(
                frame_side_data_payload.len() % FrameMotionVector::DATA_LEN,
                0
            );
            let payload_bytes = payload.to_bytes();
            assert_eq!(payload_bytes.as_slice(), frame_side_data_payload);
            assert!(!payload.is_empty());
            assert_eq!(
                payload.len(),
                frame_side_data_payload.len() / FrameMotionVector::DATA_LEN
            );
            for vector in payload.vectors() {
                assert_eq!(
                    FrameMotionVector::parse(&vector.to_bytes()).unwrap(),
                    *vector
                );
                assert_eq!(&vector.to_bytes()[14..16], &[0, 0]);
                assert_eq!(&vector.to_bytes()[34..40], &[0, 0, 0, 0, 0, 0]);
            }
            assert_eq!(FrameMotionVectors::parse(&payload_bytes).unwrap(), payload);
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::MotionVectors),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::MotionVectors);
            assert!(
                frame_side_data_payload.is_empty()
                    || !frame_side_data_payload
                        .chunks_exact(FrameMotionVector::DATA_LEN)
                        .remainder()
                        .is_empty()
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
    match frame.side_data()[0].mastering_display_metadata() {
        Ok(Some(value)) => {
            assert_eq!(
                frame_side_data_kind,
                FrameSideDataKind::MasteringDisplayMetadata
            );
            assert_eq!(
                frame_side_data_payload.len(),
                FrameMasteringDisplayMetadata::DATA_LEN
            );
            assert_eq!(value.to_bytes().as_slice(), frame_side_data_payload);
            assert_eq!(
                FrameMasteringDisplayMetadata::parse(&value.to_bytes()).unwrap(),
                value
            );
            assert_eq!(value.has_primaries(), value.has_primaries_raw() != 0);
            assert_eq!(value.has_luminance(), value.has_luminance_raw() != 0);

            let mut offset = 0;
            for primary in value.display_primaries() {
                for coordinate in primary {
                    assert_eq!(
                        read_rational_from_payload(&frame_side_data_payload, &mut offset),
                        coordinate
                    );
                }
            }
            for coordinate in value.white_point() {
                assert_eq!(
                    read_rational_from_payload(&frame_side_data_payload, &mut offset),
                    coordinate
                );
            }
            assert_eq!(
                read_rational_from_payload(&frame_side_data_payload, &mut offset),
                value.min_luminance()
            );
            assert_eq!(
                read_rational_from_payload(&frame_side_data_payload, &mut offset),
                value.max_luminance()
            );
        }
        Ok(None) => assert_ne!(
            frame_side_data_kind,
            FrameSideDataKind::MasteringDisplayMetadata
        ),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(
                frame_side_data_kind,
                FrameSideDataKind::MasteringDisplayMetadata
            );
            assert_ne!(
                frame_side_data_payload.len(),
                FrameMasteringDisplayMetadata::DATA_LEN
            );
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
    match frame.side_data()[0].spherical_mapping() {
        Ok(Some(value)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::Spherical);
            assert_eq!(
                frame_side_data_payload.len(),
                FrameSphericalMapping::DATA_LEN
            );
            assert_eq!(value.to_bytes().as_slice(), frame_side_data_payload);
            assert_eq!(
                FrameSphericalProjection::from_raw(value.projection().as_raw()).unwrap(),
                value.projection()
            );
            assert_eq!(
                FrameSphericalMapping::parse(&value.to_bytes()).unwrap(),
                value
            );
            assert_eq!(
                value.bounds(),
                [
                    value.bound_left(),
                    value.bound_top(),
                    value.bound_right(),
                    value.bound_bottom(),
                ]
            );
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::Spherical),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::Spherical);
            let raw_invalid = if frame_side_data_payload.len() == FrameSphericalMapping::DATA_LEN {
                let mut raw = [0; 4];
                raw.copy_from_slice(&frame_side_data_payload[..4]);
                FrameSphericalProjection::from_raw(i32::from_ne_bytes(raw)).is_err()
            } else {
                false
            };
            assert!(
                frame_side_data_payload.len() != FrameSphericalMapping::DATA_LEN || raw_invalid
            );
        }
    }
    match frame.side_data()[0].content_light_metadata() {
        Ok(Some(value)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::ContentLightLevel);
            assert_eq!(
                frame_side_data_payload.len(),
                FrameContentLightMetadata::DATA_LEN
            );
            assert_eq!(value.to_bytes().as_slice(), frame_side_data_payload);
            let mut max_content_light_level = [0; 4];
            max_content_light_level.copy_from_slice(&frame_side_data_payload[0..4]);
            let mut max_average_light_level = [0; 4];
            max_average_light_level.copy_from_slice(&frame_side_data_payload[4..8]);
            assert_eq!(
                value.max_content_light_level(),
                u32::from_ne_bytes(max_content_light_level)
            );
            assert_eq!(
                value.max_average_light_level(),
                u32::from_ne_bytes(max_average_light_level)
            );
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::ContentLightLevel),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::ContentLightLevel);
            assert_ne!(
                frame_side_data_payload.len(),
                FrameContentLightMetadata::DATA_LEN
            );
        }
    }
    match frame.side_data()[0].icc_profile() {
        Ok(Some(value)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::IccProfile);
            assert_eq!(value.data(), frame_side_data_payload.as_slice());
            assert_eq!(value.name(), None);
            assert_eq!(
                usize::try_from(value.declared_size()).unwrap(),
                frame_side_data_payload.len()
            );
            assert_eq!(value.data()[36..40], FrameIccProfile::ICC_SIGNATURE);
            assert!(
                FrameIccProfile::MIN_DATA_LEN
                    + usize::try_from(value.tag_count()).unwrap() * FrameIccProfile::TAG_RECORD_LEN
                    <= frame_side_data_payload.len()
            );
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::IccProfile),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::IccProfile);
            assert!(icc_profile_payload_invalid(&frame_side_data_payload));
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
    match frame.side_data()[0].dynamic_hdr_plus() {
        Ok(Some(value)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::DynamicHdrPlus);
            assert_eq!(frame_side_data_payload.len(), FrameDynamicHdrPlus::DATA_LEN);
            assert_eq!(value.data(), frame_side_data_payload.as_slice());
            assert_eq!(
                value.itu_t_t35_country_code(),
                FrameDynamicHdrPlus::ITU_T_T35_COUNTRY_CODE
            );
            assert_eq!(
                value.application_version(),
                FrameDynamicHdrPlus::APPLICATION_VERSION
            );
            assert!((1..=FrameDynamicHdrPlus::MAX_WINDOWS).contains(&value.num_windows()));
            for window in 0..value.num_windows() {
                let params = value.color_transform_params(window).unwrap();
                assert!(params.overlap_process_option().is_ok());
                assert!(
                    params.num_distribution_maxrgb_percentiles()
                        <= FrameHdrPlusColorTransformParams::MAX_DISTRIBUTION_MAXRGB_PERCENTILES
                );
                assert!(params.tone_mapping_flag() <= 1);
                assert!(
                    params.num_bezier_curve_anchors()
                        <= FrameHdrPlusColorTransformParams::MAX_BEZIER_CURVE_ANCHORS
                );
                assert!(params.color_saturation_mapping_flag() <= 1);
            }
            assert!(value.targeted_system_display_actual_peak_luminance_flag() <= 1);
            assert!(value.mastering_display_actual_peak_luminance_flag() <= 1);
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::DynamicHdrPlus),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::DynamicHdrPlus);
            assert!(dynamic_hdr_plus_payload_invalid(&frame_side_data_payload));
        }
    }
    match frame.side_data()[0].regions_of_interest() {
        Ok(Some(value)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::RegionsOfInterest);
            assert_eq!(value.to_bytes(), frame_side_data_payload);
            assert_eq!(
                frame_side_data_payload.len(),
                value.len() * FrameRegionOfInterest::DATA_LEN
            );
            assert!(!value.is_empty());
            for region in value.regions() {
                assert_eq!(region.self_size(), FrameRegionOfInterest::SELF_SIZE);
                assert!(region_qoffset_valid(region.qoffset()));
            }
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::RegionsOfInterest),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::RegionsOfInterest);
            assert!(regions_of_interest_payload_invalid(
                &frame_side_data_payload
            ));
        }
    }
    match frame.side_data()[0].video_enc_params() {
        Ok(Some(value)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::VideoEncParams);
            assert_eq!(value.to_bytes(), frame_side_data_payload);
            assert_eq!(
                frame_side_data_payload.len(),
                FrameVideoEncParams::HEADER_LEN
                    + value.nb_blocks() * FrameVideoBlockParams::DATA_LEN
            );
            for block in value.blocks() {
                assert!(block.width() > 0);
                assert!(block.height() > 0);
            }
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::VideoEncParams),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::VideoEncParams);
            assert!(video_enc_params_payload_invalid(&frame_side_data_payload));
        }
    }
    match frame.side_data()[0].video_hint() {
        Ok(Some(value)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::VideoHint);
            assert_eq!(value.to_bytes(), frame_side_data_payload);
            assert_eq!(
                frame_side_data_payload.len(),
                FrameVideoHint::HEADER_LEN + value.nb_rects() * FrameVideoRect::DATA_LEN
            );
            assert!(matches!(
                value.hint_type(),
                FrameVideoHintType::Constant | FrameVideoHintType::Changed
            ));
            for rect in value.rects() {
                assert!(rect.width() > 0);
                assert!(rect.height() > 0);
                assert!(rect.x().checked_add(rect.width()).is_some());
                assert!(rect.y().checked_add(rect.height()).is_some());
            }
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::VideoHint),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::VideoHint);
            assert!(video_hint_payload_invalid(&frame_side_data_payload));
        }
    }
    match frame.side_data()[0].lcevc() {
        Some(value) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::Lcevc);
            assert_eq!(value.data(), frame_side_data_payload.as_slice());
            assert_eq!(value.is_empty(), frame_side_data_payload.is_empty());
        }
        None => assert_ne!(frame_side_data_kind, FrameSideDataKind::Lcevc),
    }
    match frame.side_data()[0].view_id() {
        Ok(Some(value)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::ViewId);
            assert_eq!(frame_side_data_payload.len(), FrameViewId::DATA_LEN);
            assert_eq!(
                value.to_bytes().as_slice(),
                frame_side_data_payload.as_slice()
            );
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::ViewId),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::ViewId);
            assert_ne!(frame_side_data_payload.len(), FrameViewId::DATA_LEN);
        }
    }
    match frame.side_data()[0].three_d_reference_displays() {
        Ok(Some(value)) => {
            assert_eq!(
                frame_side_data_kind,
                FrameSideDataKind::ThreeDReferenceDisplays
            );
            assert_eq!(value.to_bytes(), frame_side_data_payload);
            assert!(
                (1..=FrameThreeDReferenceDisplays::MAX_REF_DISPLAYS).contains(&value.nb_displays())
            );
            assert!(value.prec_ref_display_width() <= 31);
            assert!(value.prec_ref_viewing_dist() <= 31);
            assert_eq!(
                frame_side_data_payload.len(),
                FrameThreeDReferenceDisplays::ENTRIES_OFFSET
                    + value.nb_displays() * FrameThreeDReferenceDisplay::DATA_LEN
            );
        }
        Ok(None) => assert_ne!(
            frame_side_data_kind,
            FrameSideDataKind::ThreeDReferenceDisplays
        ),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(
                frame_side_data_kind,
                FrameSideDataKind::ThreeDReferenceDisplays
            );
            assert!(three_d_reference_displays_payload_invalid(
                &frame_side_data_payload
            ));
        }
    }
    match frame.side_data()[0].exif() {
        Ok(Some(value)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::Exif);
            assert_eq!(value.data(), frame_side_data_payload.as_slice());
            assert!(!exif_payload_invalid(&frame_side_data_payload));
            assert!(value.first_ifd_offset() >= FrameExif::TIFF_HEADER_LEN);
            assert!(value.ifd_count() <= FrameExif::MAX_IFDS);
            assert!(value.linked_ifd_count() <= FrameExif::MAX_LINKED_IFDS);
            for ifd in value.ifds() {
                assert!(ifd.offset() >= FrameExif::TIFF_HEADER_LEN);
                assert!(ifd.entry_count() <= FrameExif::MAX_IFD_ENTRIES);
                if let Some(next) = ifd.next_ifd_offset() {
                    assert!(next >= FrameExif::TIFF_HEADER_LEN);
                }
                for entry in ifd.entries() {
                    assert!((1..=13).contains(&entry.tiff_type().raw()));
                    assert_eq!(
                        entry
                            .tiff_type()
                            .element_size()
                            .checked_mul(entry.count() as usize)
                            .unwrap(),
                        entry.data_len()
                    );
                    assert_eq!(entry.value_or_offset_bytes().len(), 4);
                    if let Some((start, end)) = entry.data_range() {
                        assert!(!entry.is_inline());
                        assert!(start <= end);
                        assert!(end <= frame_side_data_payload.len());
                        assert_eq!(end - start, entry.data_len());
                        assert_eq!(entry.value_data(), &frame_side_data_payload[start..end]);
                    } else {
                        assert!(entry.is_inline());
                        assert!(entry.data_len() <= 4);
                        assert_eq!(entry.value_data().len(), entry.data_len());
                    }
                    exercise_exif_entry_typed_values(*entry);
                }
            }
            for linked in value.linked_ifds() {
                assert!(matches!(
                    linked.kind(),
                    FrameExifIfdPointerKind::Exif
                        | FrameExifIfdPointerKind::Gps
                        | FrameExifIfdPointerKind::Interoperability
                ));
                assert_eq!(linked.kind().tag(), linked.source_tag());
                assert!(linked.parent_ifd_offset() >= FrameExif::TIFF_HEADER_LEN);
                assert!(linked.offset() >= FrameExif::TIFF_HEADER_LEN);
                assert!(linked.ifd().entry_count() <= FrameExif::MAX_IFD_ENTRIES);
                assert!(value.linked_ifd(linked.kind()).is_some());
                for entry in linked.ifd().entries() {
                    exercise_exif_entry_typed_values(*entry);
                }
            }
            match value.common_tags() {
                Ok(common) => {
                    if let Some(value) = common.exposure_time() {
                        assert_ne!(value.denominator(), 0);
                    }
                    if let Some(value) = common.f_number() {
                        assert_ne!(value.denominator(), 0);
                    }
                    if let Some(value) = common.exposure_bias_value() {
                        assert_ne!(value.denominator(), 0);
                    }
                    if let Some(value) = common.focal_length() {
                        assert_ne!(value.denominator(), 0);
                    }
                    if let Some(value) = common.digital_zoom_ratio() {
                        assert_ne!(value.denominator(), 0);
                    }
                    if let Some(latitude) = common.gps_latitude() {
                        assert!(latitude.iter().all(|value| value.denominator() != 0));
                    }
                    if let Some(longitude) = common.gps_longitude() {
                        assert!(longitude.iter().all(|value| value.denominator() != 0));
                    }
                    if let Some(value) = common.gps_altitude() {
                        assert_ne!(value.denominator(), 0);
                    }
                    if let Some(time_stamp) = common.gps_time_stamp() {
                        assert!(time_stamp.iter().all(|value| value.denominator() != 0));
                    }
                    if let Some(value) = common.gps_dop() {
                        assert_ne!(value.denominator(), 0);
                    }
                    if let Some(value) = common.gps_speed() {
                        assert_ne!(value.denominator(), 0);
                    }
                    if let Some(value) = common.gps_track() {
                        assert_ne!(value.denominator(), 0);
                    }
                    if let Some(value) = common.gps_img_direction() {
                        assert_ne!(value.denominator(), 0);
                    }
                    if let Some(latitude) = common.gps_dest_latitude() {
                        assert!(latitude.iter().all(|value| value.denominator() != 0));
                    }
                    if let Some(longitude) = common.gps_dest_longitude() {
                        assert!(longitude.iter().all(|value| value.denominator() != 0));
                    }
                    if let Some(value) = common.gps_dest_bearing() {
                        assert_ne!(value.denominator(), 0);
                    }
                    if let Some(value) = common.gps_dest_distance() {
                        assert_ne!(value.denominator(), 0);
                    }
                }
                Err(err) => assert_eq!(err.kind(), AvErrorKind::InvalidData),
            }
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::Exif),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::Exif);
            assert!(exif_payload_invalid(&frame_side_data_payload));
        }
    }
    match frame.side_data()[0].film_grain_params() {
        Ok(Some(value)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::FilmGrainParams);
            assert_eq!(value.data(), frame_side_data_payload.as_slice());
            assert!(!film_grain_params_payload_invalid(&frame_side_data_payload));
            match value.params_type() {
                FrameFilmGrainParamsType::None => {}
                FrameFilmGrainParamsType::Av1 => {
                    let aom = value.aom_params().unwrap().unwrap();
                    assert!(aom.num_y_points() <= FrameFilmGrainAomParams::Y_POINTS);
                    assert!(aom.num_uv_points(0).unwrap() <= FrameFilmGrainAomParams::UV_POINTS);
                    assert!(aom.num_uv_points(1).unwrap() <= FrameFilmGrainAomParams::UV_POINTS);
                    assert!(aom.ar_coeff_count_y() <= FrameFilmGrainAomParams::AR_COEFFS_Y);
                    assert!(aom.ar_coeff_count_uv() <= FrameFilmGrainAomParams::AR_COEFFS_UV);
                }
                FrameFilmGrainParamsType::H274 => {
                    let h274 = value.h274_params().unwrap().unwrap();
                    for component in 0..FrameFilmGrainH274Params::COMPONENTS {
                        if h274.component_model_present(component).unwrap() {
                            assert!((1..=FrameFilmGrainH274Params::MAX_INTENSITY_INTERVALS)
                                .contains(&h274.num_intensity_intervals(component).unwrap()));
                            assert!((1..=FrameFilmGrainH274Params::MAX_MODEL_VALUES)
                                .contains(&h274.num_model_values(component).unwrap()));
                        }
                    }
                }
            }
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::FilmGrainParams),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::FilmGrainParams);
            assert!(film_grain_params_payload_invalid(&frame_side_data_payload));
        }
    }
    match frame.side_data()[0].detection_bboxes() {
        Ok(Some(value)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::DetectionBboxes);
            assert_eq!(value.data(), frame_side_data_payload.as_slice());
            assert!(!detection_bboxes_payload_invalid(&frame_side_data_payload));
            assert_eq!(
                frame_side_data_payload.len(),
                FrameDetectionBboxes::HEADER_LEN + value.nb_bboxes() * FrameDetectionBbox::DATA_LEN
            );
            for bbox in value.bboxes() {
                let bbox = bbox.unwrap();
                assert!(bbox.classify_count() <= FrameDetectionBbox::MAX_CLASSIFICATIONS);
                assert_eq!(bbox.detect_label_raw().len(), FrameDetectionBbox::LABEL_LEN);
                assert!(bbox.detect_label().len() <= FrameDetectionBbox::LABEL_LEN);
                for index in 0..bbox.classify_count() {
                    assert!(bbox.classify_label(index).is_some());
                    assert!(bbox.classify_confidence(index).is_some());
                }
                assert!(bbox
                    .classify_label(FrameDetectionBbox::MAX_CLASSIFICATIONS)
                    .is_none());
            }
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::DetectionBboxes),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::DetectionBboxes);
            assert!(detection_bboxes_payload_invalid(&frame_side_data_payload));
        }
    }
    match frame.side_data()[0].dolby_vision_rpu_buffer() {
        Some(value) => {
            assert_eq!(
                frame_side_data_kind,
                FrameSideDataKind::DolbyVisionRpuBuffer
            );
            assert_eq!(value.data(), frame_side_data_payload.as_slice());
            assert_eq!(value.is_empty(), frame_side_data_payload.is_empty());
        }
        None => assert_ne!(
            frame_side_data_kind,
            FrameSideDataKind::DolbyVisionRpuBuffer
        ),
    }
    match frame.side_data()[0].dolby_vision_metadata() {
        Ok(Some(value)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::DolbyVisionMetadata);
            assert_eq!(value.data(), frame_side_data_payload.as_slice());
            assert!(!dolby_vision_metadata_payload_invalid(
                &frame_side_data_payload
            ));
            assert!(value.num_ext_blocks() <= FrameDolbyVisionMetadata::MAX_EXT_BLOCKS);
            assert_eq!(
                frame_side_data_payload.len(),
                FrameDolbyVisionMetadata::DATA_LEN
            );
            assert!(value.header().is_ok());
            assert!(value.mapping().is_ok());
            assert!(value.color().is_ok());
            assert_eq!(value.ext_blocks().count(), value.num_ext_blocks());
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::DolbyVisionMetadata),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::DolbyVisionMetadata);
            assert!(dolby_vision_metadata_payload_invalid(
                &frame_side_data_payload
            ));
        }
    }
    match frame.side_data()[0].dynamic_hdr_vivid() {
        Ok(Some(value)) => {
            assert_eq!(frame_side_data_kind, FrameSideDataKind::DynamicHdrVivid);
            assert_eq!(value.data(), frame_side_data_payload.as_slice());
            assert!(!dynamic_hdr_vivid_payload_invalid(&frame_side_data_payload));
            assert!((FrameDynamicHdrVivid::MIN_SYSTEM_START_CODE
                ..=FrameDynamicHdrVivid::MAX_SYSTEM_START_CODE)
                .contains(&value.system_start_code()));
            assert!((1..=FrameDynamicHdrVivid::MAX_WINDOWS).contains(&value.num_windows()));
            for index in 0..value.num_windows() {
                let params = value.color_transform_params(index).unwrap();
                assert_eq!(
                    params.data().len(),
                    FrameHdrVividColorTransformParams::DATA_LEN
                );
                assert!(matches!(params.tone_mapping_mode_flag(), 0 | 1));
                assert!(matches!(params.color_saturation_mapping_flag(), 0 | 1));
                if params.tone_mapping_mode_flag() == 1 {
                    assert!(
                        (1..=FrameHdrVividColorTransformParams::MAX_TONE_MAPPING_PARAMS)
                            .contains(&params.tone_mapping_param_num())
                    );
                    for tm_index in 0..params.tone_mapping_param_num() {
                        let tm = params.tone_mapping_params(tm_index).unwrap();
                        assert!(matches!(tm.base_enable_flag(), 0 | 1));
                        assert!(matches!(tm.three_spline_enable_flag(), 0 | 1));
                        if tm.three_spline_enable_flag() == 1 {
                            assert!((1..=FrameHdrVividColorToneMappingParams::MAX_THREE_SPLINES)
                                .contains(&tm.three_spline_num()));
                            for spline_index in 0..tm.three_spline_num() {
                                let spline = tm.three_spline(spline_index).unwrap().unwrap();
                                assert!((0..=3).contains(&spline.th_mode()));
                            }
                        }
                    }
                }
                if params.color_saturation_mapping_flag() == 1 {
                    assert!(
                        params.color_saturation_num()
                            <= FrameHdrVividColorTransformParams::MAX_COLOR_SATURATION_GAINS
                    );
                }
            }
        }
        Ok(None) => assert_ne!(frame_side_data_kind, FrameSideDataKind::DynamicHdrVivid),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(frame_side_data_kind, FrameSideDataKind::DynamicHdrVivid);
            assert!(dynamic_hdr_vivid_payload_invalid(&frame_side_data_payload));
        }
    }
    match frame.side_data()[0].ambient_viewing_environment() {
        Ok(Some(value)) => {
            assert_eq!(
                frame_side_data_kind,
                FrameSideDataKind::AmbientViewingEnvironment
            );
            assert_eq!(
                frame_side_data_payload.len(),
                FrameAmbientViewingEnvironment::DATA_LEN
            );
            assert_eq!(value.to_bytes().as_slice(), frame_side_data_payload);
            assert_eq!(
                FrameAmbientViewingEnvironment::parse(value.to_bytes().as_slice()).unwrap(),
                value
            );
            assert!(ambient_nonnegative_rational(value.ambient_illuminance()));
            assert!(ambient_unit_interval_rational(value.ambient_light_x()));
            assert!(ambient_unit_interval_rational(value.ambient_light_y()));
        }
        Ok(None) => assert_ne!(
            frame_side_data_kind,
            FrameSideDataKind::AmbientViewingEnvironment
        ),
        Err(err) => {
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
            assert_eq!(
                frame_side_data_kind,
                FrameSideDataKind::AmbientViewingEnvironment
            );
            assert!(ambient_viewing_environment_payload_invalid(
                &frame_side_data_payload
            ));
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

    let pan_scan = FramePanScan::new(7, 1920 * 16, 1080 * 16, [[0, 0], [16, -32], [-48, 64]]);
    let pan_scan_side_data = FrameSideData::new_pan_scan(pan_scan).unwrap();
    assert_eq!(pan_scan_side_data.kind_id(), &FrameSideDataKind::PanScan);
    assert_eq!(pan_scan_side_data.pan_scan().unwrap(), Some(pan_scan));
    assert_eq!(pan_scan_side_data.data(), &pan_scan.to_bytes()[..]);
    assert_eq!(pan_scan.field_position(1), Some([16, -32]));
    assert_eq!(pan_scan.field_position(FramePanScan::POSITIONS), None);
    assert_eq!(
        FramePanScan::parse(&[0; FramePanScan::DATA_LEN - 1])
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_pan_scan =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0; 24]).unwrap();
    assert_eq!(non_pan_scan.pan_scan().unwrap(), None);

    let a53_payload = vec![0x04, 0xF8, 0x2A, 0x05, 0x43, 0x21];
    let a53_side_data = FrameSideData::new_a53_closed_captions(a53_payload.clone()).unwrap();
    assert_eq!(
        a53_side_data.kind_id(),
        &FrameSideDataKind::A53ClosedCaptions
    );
    let a53 = a53_side_data.a53_closed_captions().unwrap().unwrap();
    assert_eq!(a53.data(), a53_payload.as_slice());
    assert_eq!(a53.entry_count(), 2);
    assert_eq!(a53.entry(0), Some([0x04, 0xF8, 0x2A]));
    assert_eq!(a53.entry(1), Some([0x05, 0x43, 0x21]));
    assert_eq!(a53.entry(2), None);
    assert_eq!(
        a53.entries().collect::<Vec<_>>(),
        vec![[0x04, 0xF8, 0x2A], [0x05, 0x43, 0x21]]
    );
    assert!(FrameSideData::new_a53_closed_captions(Vec::new())
        .unwrap()
        .a53_closed_captions()
        .unwrap()
        .unwrap()
        .is_empty());
    assert_eq!(
        FrameA53ClosedCaptions::parse(&[0; FrameA53ClosedCaptions::BYTES_PER_CC - 1])
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_a53 =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0; 3]).unwrap();
    assert_eq!(non_a53.a53_closed_captions().unwrap(), None);

    let stereo3d = FrameStereo3d::new(
        FrameStereo3dType::SideBySide,
        FrameStereo3dFlags::INVERT,
        FrameStereo3dView::Right,
        FrameStereo3dPrimaryEye::Left,
        63_500,
        Rational::from_raw(-1, 2),
        Rational::from_raw(90, 1),
    )
    .unwrap();
    let stereo3d_side_data = FrameSideData::new_stereo3d(stereo3d).unwrap();
    assert_eq!(stereo3d_side_data.kind_id(), &FrameSideDataKind::Stereo3d);
    assert_eq!(stereo3d_side_data.stereo3d().unwrap(), Some(stereo3d));
    assert_eq!(
        FrameStereo3d::parse(&stereo3d.to_bytes()).unwrap(),
        stereo3d
    );
    assert_eq!(
        stereo3d.stereo_type().ffmpeg_constant(),
        "AV_STEREO3D_SIDEBYSIDE"
    );
    assert_eq!(stereo3d.view().ffmpeg_constant(), "AV_STEREO3D_VIEW_RIGHT");
    assert_eq!(
        stereo3d.primary_eye().ffmpeg_constant(),
        "AV_PRIMARY_EYE_LEFT"
    );
    assert!(stereo3d.has_inverted_views());
    assert_eq!(
        FrameStereo3d::parse(&[0; FrameStereo3d::DATA_LEN - 1])
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let mut bad_stereo3d = stereo3d.to_bytes();
    write_ne_i32(&mut bad_stereo3d, FrameStereo3d::TYPE_OFFSET, 9);
    assert_eq!(
        FrameStereo3d::parse(&bad_stereo3d).unwrap_err().kind(),
        AvErrorKind::InvalidData
    );
    let mut bad_stereo3d = stereo3d.to_bytes();
    write_ne_rational(
        &mut bad_stereo3d,
        FrameStereo3d::HORIZONTAL_DISPARITY_ADJUSTMENT_OFFSET,
        Rational::from_raw(2, 1),
    );
    assert_eq!(
        FrameStereo3d::parse(&bad_stereo3d).unwrap_err().kind(),
        AvErrorKind::InvalidData
    );
    let unset_stereo3d = FrameStereo3d::new(
        FrameStereo3dType::TwoDimensional,
        FrameStereo3dFlags::EMPTY,
        FrameStereo3dView::Packed,
        FrameStereo3dPrimaryEye::None,
        0,
        Rational::from_raw(0, 0),
        Rational::from_raw(0, 0),
    )
    .unwrap();
    assert_eq!(
        FrameStereo3d::parse(&unset_stereo3d.to_bytes()).unwrap(),
        unset_stereo3d
    );
    let non_stereo =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0; 36]).unwrap();
    assert_eq!(non_stereo.stereo3d().unwrap(), None);

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
        std::f64::consts::FRAC_1_SQRT_2,
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

    let replay_gain = FrameReplayGain::new(
        -650_000,
        100_000,
        FrameReplayGain::GAIN_UNKNOWN,
        FrameReplayGain::PEAK_UNKNOWN,
    );
    let replay_gain_side_data = FrameSideData::new_replay_gain(replay_gain).unwrap();
    assert_eq!(
        replay_gain_side_data.kind_id(),
        &FrameSideDataKind::ReplayGain
    );
    assert_eq!(
        replay_gain_side_data.replay_gain().unwrap(),
        Some(replay_gain)
    );
    assert_eq!(replay_gain_side_data.data(), &replay_gain.to_bytes()[..]);
    assert_eq!(replay_gain.track_gain(), -650_000);
    assert_eq!(replay_gain.track_peak(), 100_000);
    assert!(replay_gain.album_gain_unknown());
    assert!(replay_gain.album_peak_unknown());
    assert_eq!(
        FrameSideData::new_with_kind(FrameSideDataKind::ReplayGain, vec![0; 15])
            .unwrap()
            .replay_gain()
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_replay_gain =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0; 16]).unwrap();
    assert_eq!(non_replay_gain.replay_gain().unwrap(), None);

    let motion_vectors = FrameMotionVectors::new(vec![
        FrameMotionVector::new(-1, 16, 8, -20, 32, 64, -12, 0, 1200, -3400, 4),
        FrameMotionVector::new(1, 4, 4, 320, -240, -64, 96, u64::MAX, -128, 256, 2),
    ])
    .unwrap();
    let motion_side_data = FrameSideData::new_motion_vectors(motion_vectors.clone()).unwrap();
    assert_eq!(
        motion_side_data.kind_id(),
        &FrameSideDataKind::MotionVectors
    );
    assert_eq!(
        motion_side_data.motion_vectors().unwrap(),
        Some(motion_vectors.clone())
    );
    assert_eq!(
        motion_side_data.data(),
        motion_vectors.to_bytes().as_slice()
    );
    assert_eq!(motion_vectors.vectors()[0].source(), -1);
    assert_eq!(motion_vectors.vectors()[0].width(), 16);
    assert_eq!(motion_vectors.vectors()[0].height(), 8);
    assert_eq!(motion_vectors.vectors()[0].motion_scale(), 4);
    assert_eq!(
        FrameSideData::new_with_kind(FrameSideDataKind::MotionVectors, vec![0; 39])
            .unwrap()
            .motion_vectors()
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_motion_vectors =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0; 40]).unwrap();
    assert_eq!(non_motion_vectors.motion_vectors().unwrap(), None);

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

    let mastering = FrameMasteringDisplayMetadata::new(
        [
            [
                Rational::from_raw(34_000, 50_000),
                Rational::from_raw(16_000, 50_000),
            ],
            [
                Rational::from_raw(13_250, 50_000),
                Rational::from_raw(34_500, 50_000),
            ],
            [
                Rational::from_raw(7_500, 50_000),
                Rational::from_raw(3_000, 50_000),
            ],
        ],
        [
            Rational::from_raw(15_635, 50_000),
            Rational::from_raw(16_450, 50_000),
        ],
        Rational::from_raw(50, 10_000),
        Rational::from_raw(1000, 1),
        1,
        -2,
    );
    let mastering_side_data = FrameSideData::new_mastering_display_metadata(mastering).unwrap();
    assert_eq!(
        mastering_side_data.kind_id(),
        &FrameSideDataKind::MasteringDisplayMetadata
    );
    assert_eq!(
        mastering_side_data.mastering_display_metadata().unwrap(),
        Some(mastering)
    );
    assert_eq!(mastering_side_data.data(), &mastering.to_bytes()[..]);
    assert!(mastering.has_primaries());
    assert!(mastering.has_luminance());
    assert_eq!(mastering.has_luminance_raw(), -2);
    assert_eq!(
        FrameSideData::new_with_kind(FrameSideDataKind::MasteringDisplayMetadata, vec![0; 87])
            .unwrap()
            .mastering_display_metadata()
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_mastering =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0; 88]).unwrap();
    assert_eq!(non_mastering.mastering_display_metadata().unwrap(), None);

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

    let spherical = FrameSphericalMapping::new(
        FrameSphericalProjection::EquirectangularTile,
        45 << 16,
        -10 << 16,
        5 << 16,
        [10, 20, 30, 40],
        7,
    );
    let spherical_side_data = FrameSideData::new_spherical_mapping(spherical).unwrap();
    assert_eq!(spherical_side_data.kind_id(), &FrameSideDataKind::Spherical);
    assert_eq!(
        spherical_side_data.spherical_mapping().unwrap(),
        Some(spherical)
    );
    assert_eq!(spherical_side_data.data(), &spherical.to_bytes()[..]);
    assert_eq!(
        FrameSphericalProjection::from_raw(6).unwrap(),
        FrameSphericalProjection::ParametricImmersive
    );
    assert_eq!(
        FrameSphericalProjection::ParametricImmersive.ffmpeg_constant(),
        "AV_SPHERICAL_PARAMETRIC_IMMERSIVE"
    );
    assert_eq!(
        FrameSphericalProjection::from_raw(7).unwrap_err().kind(),
        AvErrorKind::InvalidData
    );
    assert_eq!(
        FrameSphericalMapping::parse(&[0; FrameSphericalMapping::DATA_LEN - 1])
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let mut invalid_spherical = [0; FrameSphericalMapping::DATA_LEN];
    invalid_spherical[0..4].copy_from_slice(&7i32.to_ne_bytes());
    assert_eq!(
        FrameSphericalMapping::parse(&invalid_spherical)
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_spherical =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0; 36]).unwrap();
    assert_eq!(non_spherical.spherical_mapping().unwrap(), None);

    let content_light = FrameContentLightMetadata::new(1000, 400);
    let content_light_side_data = FrameSideData::new_content_light_metadata(content_light).unwrap();
    assert_eq!(
        content_light_side_data.kind_id(),
        &FrameSideDataKind::ContentLightLevel
    );
    assert_eq!(
        content_light_side_data.content_light_metadata().unwrap(),
        Some(content_light)
    );
    assert_eq!(
        content_light_side_data.data(),
        &content_light.to_bytes()[..]
    );
    assert_eq!(content_light.max_content_light_level(), 1000);
    assert_eq!(content_light.max_average_light_level(), 400);
    assert_eq!(
        FrameContentLightMetadata::parse(&FrameContentLightMetadata::new(u32::MAX, 0).to_bytes())
            .unwrap()
            .max_content_light_level(),
        u32::MAX
    );
    assert_eq!(
        FrameContentLightMetadata::parse(&[0; FrameContentLightMetadata::DATA_LEN - 1])
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_content_light =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0; 8]).unwrap();
    assert_eq!(non_content_light.content_light_metadata().unwrap(), None);

    let icc_profile = minimal_icc_profile_fixture();
    let icc_side_data =
        FrameSideData::new_icc_profile(icc_profile.clone(), Some("display-p3")).unwrap();
    let parsed_icc = icc_side_data.icc_profile().unwrap().unwrap();
    assert_eq!(icc_side_data.kind_id(), &FrameSideDataKind::IccProfile);
    assert_eq!(parsed_icc.data(), icc_profile.as_slice());
    assert_eq!(parsed_icc.name(), Some("display-p3"));
    assert_eq!(
        parsed_icc.declared_size(),
        FrameIccProfile::MIN_DATA_LEN as u32
    );
    assert_eq!(parsed_icc.profile_version_raw(), 0x0430_0000);
    assert_eq!(parsed_icc.device_class(), *b"mntr");
    assert_eq!(parsed_icc.color_space(), *b"RGB ");
    assert_eq!(parsed_icc.profile_connection_space(), *b"XYZ ");
    assert_eq!(parsed_icc.tag_count(), 0);
    assert_eq!(
        FrameSideData::new_icc_profile(Vec::new(), None)
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let mut bad_icc = icc_profile.clone();
    bad_icc[36..40].copy_from_slice(b"bad!");
    assert_eq!(
        FrameSideData::new_with_kind(FrameSideDataKind::IccProfile, bad_icc)
            .unwrap()
            .icc_profile()
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_icc =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, icc_profile).unwrap();
    assert_eq!(non_icc.icc_profile().unwrap(), None);

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

    let dynamic_hdr_plus = minimal_dynamic_hdr_plus_fixture();
    let dynamic_hdr_plus_side_data =
        FrameSideData::new_dynamic_hdr_plus(dynamic_hdr_plus.clone()).unwrap();
    let parsed_dynamic_hdr_plus = dynamic_hdr_plus_side_data
        .dynamic_hdr_plus()
        .unwrap()
        .unwrap();
    assert_eq!(
        dynamic_hdr_plus_side_data.kind_id(),
        &FrameSideDataKind::DynamicHdrPlus
    );
    assert_eq!(parsed_dynamic_hdr_plus.data(), dynamic_hdr_plus.as_slice());
    assert_eq!(
        parsed_dynamic_hdr_plus.itu_t_t35_country_code(),
        FrameDynamicHdrPlus::ITU_T_T35_COUNTRY_CODE
    );
    assert_eq!(
        parsed_dynamic_hdr_plus.application_version(),
        FrameDynamicHdrPlus::APPLICATION_VERSION
    );
    assert_eq!(parsed_dynamic_hdr_plus.num_windows(), 1);
    assert_eq!(parsed_dynamic_hdr_plus.color_transform_params(1), None);
    assert_eq!(
        parsed_dynamic_hdr_plus
            .color_transform_params(0)
            .unwrap()
            .overlap_process_option()
            .unwrap(),
        FrameHdrPlusOverlapProcessOption::WeightedAveraging
    );
    assert_eq!(
        FrameSideData::new_dynamic_hdr_plus(Vec::new())
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let mut bad_hdr_plus = dynamic_hdr_plus.clone();
    bad_hdr_plus[0] = 0;
    assert_eq!(
        FrameSideData::new_with_kind(FrameSideDataKind::DynamicHdrPlus, bad_hdr_plus)
            .unwrap()
            .dynamic_hdr_plus()
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_dynamic_hdr_plus =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, dynamic_hdr_plus).unwrap();
    assert_eq!(non_dynamic_hdr_plus.dynamic_hdr_plus().unwrap(), None);

    let roi = FrameRegionOfInterest::new(0, 16, 4, 20, Rational::from_raw(-1, 10)).unwrap();
    let rois = FrameRegionsOfInterest::new(vec![roi]).unwrap();
    let roi_side_data = FrameSideData::new_regions_of_interest(rois.clone()).unwrap();
    assert_eq!(
        roi_side_data.kind_id(),
        &FrameSideDataKind::RegionsOfInterest
    );
    assert_eq!(roi_side_data.data(), rois.to_bytes().as_slice());
    assert_eq!(roi_side_data.regions_of_interest().unwrap(), Some(rois));
    assert_eq!(
        FrameRegionsOfInterest::parse(&[0; FrameRegionOfInterest::DATA_LEN - 1])
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let mut bad_roi = roi.to_bytes();
    write_ne_u32(&mut bad_roi, 0, FrameRegionOfInterest::SELF_SIZE + 4);
    assert_eq!(
        FrameSideData::new_with_kind(FrameSideDataKind::RegionsOfInterest, bad_roi.to_vec())
            .unwrap()
            .regions_of_interest()
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_roi = FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0]).unwrap();
    assert_eq!(non_roi.regions_of_interest().unwrap(), None);

    let video_block = FrameVideoBlockParams::new(-2, 4, 16, 16, -1).unwrap();
    let video_enc_params = FrameVideoEncParams::new(
        FrameVideoEncParamsType::H264,
        24,
        [[0, 1], [2, 3], [4, 5], [6, 7]],
        vec![video_block],
    )
    .unwrap();
    let video_enc_side_data =
        FrameSideData::new_video_enc_params(video_enc_params.clone()).unwrap();
    assert_eq!(
        video_enc_side_data.kind_id(),
        &FrameSideDataKind::VideoEncParams
    );
    assert_eq!(video_enc_side_data.data(), video_enc_params.to_bytes());
    assert_eq!(
        video_enc_side_data.video_enc_params().unwrap(),
        Some(video_enc_params)
    );
    assert_eq!(
        FrameVideoEncParams::parse(&[0; FrameVideoEncParams::HEADER_LEN - 1])
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let mut bad_video_enc = video_enc_side_data.data().to_vec();
    write_ne_usize(
        &mut bad_video_enc,
        video_enc_params_blocks_offset_field_offset(),
        FrameVideoEncParams::HEADER_LEN - 4,
    );
    assert_eq!(
        FrameSideData::new_with_kind(FrameSideDataKind::VideoEncParams, bad_video_enc)
            .unwrap()
            .video_enc_params()
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_video_enc =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0]).unwrap();
    assert_eq!(non_video_enc.video_enc_params().unwrap(), None);

    let video_rect = FrameVideoRect::new(4, 8, 16, 32).unwrap();
    let video_hint = FrameVideoHint::new(FrameVideoHintType::Changed, vec![video_rect]).unwrap();
    let video_hint_side_data = FrameSideData::new_video_hint(video_hint.clone()).unwrap();
    assert_eq!(
        video_hint_side_data.kind_id(),
        &FrameSideDataKind::VideoHint
    );
    let parsed_video_hint = video_hint_side_data.video_hint().unwrap().unwrap();
    assert_eq!(parsed_video_hint, video_hint);
    assert_eq!(parsed_video_hint.hint_type(), FrameVideoHintType::Changed);
    assert_eq!(
        parsed_video_hint.hint_type().ffmpeg_constant(),
        "AV_VIDEO_HINT_TYPE_CHANGED"
    );
    assert_eq!(parsed_video_hint.rect(0), Some(video_rect));
    assert_eq!(parsed_video_hint.rect(1), None);
    assert_eq!(video_hint_side_data.data(), video_hint.to_bytes());
    assert_eq!(
        FrameVideoHint::parse(&[0; FrameVideoHint::HEADER_LEN - 1])
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let mut bad_video_hint = video_hint.to_bytes();
    write_ne_i32(&mut bad_video_hint, video_hint_type_field_offset(), 2);
    assert_eq!(
        FrameSideData::new_with_kind(FrameSideDataKind::VideoHint, bad_video_hint)
            .unwrap()
            .video_hint()
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    assert_eq!(
        FrameVideoRect::new(u32::MAX, 0, 1, 1).unwrap_err().kind(),
        AvErrorKind::InvalidArgument
    );
    let non_video_hint =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0]).unwrap();
    assert_eq!(non_video_hint.video_hint().unwrap(), None);

    let lcevc_bytes = vec![0x00, 0x00, 0x01, 0x7E, 0xAA, 0x00, 0x00, 0x03, 0x01];
    let lcevc_side_data = FrameSideData::new_lcevc(lcevc_bytes.clone()).unwrap();
    assert_eq!(lcevc_side_data.kind_id(), &FrameSideDataKind::Lcevc);
    let parsed_lcevc = lcevc_side_data.lcevc().unwrap();
    assert_eq!(
        FrameLcevc::parse(&lcevc_bytes).data(),
        lcevc_bytes.as_slice()
    );
    assert_eq!(parsed_lcevc.data(), lcevc_bytes.as_slice());
    assert!(!parsed_lcevc.is_empty());
    assert!(FrameLcevc::parse(&[]).is_empty());
    let non_lcevc =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0]).unwrap();
    assert_eq!(non_lcevc.lcevc(), None);

    let view_id = FrameViewId::new(-7);
    let view_id_side_data = FrameSideData::new_view_id(view_id).unwrap();
    assert_eq!(view_id_side_data.kind_id(), &FrameSideDataKind::ViewId);
    assert_eq!(
        view_id_side_data.view_id().unwrap().unwrap().as_raw(),
        view_id.as_raw()
    );
    assert_eq!(
        FrameViewId::parse(&view_id.to_bytes()).unwrap().to_bytes(),
        view_id.to_bytes()
    );
    for data in [Vec::new(), vec![0; 3], vec![0; 5]] {
        assert_eq!(
            FrameViewId::parse(&data).unwrap_err().kind(),
            AvErrorKind::InvalidData
        );
        let invalid_view_id =
            FrameSideData::new_with_kind(FrameSideDataKind::ViewId, data).unwrap();
        assert_eq!(
            invalid_view_id.view_id().unwrap_err().kind(),
            AvErrorKind::InvalidData
        );
    }
    let non_view_id =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0]).unwrap();
    assert_eq!(non_view_id.view_id().unwrap(), None);

    let tdrdi_display = FrameThreeDReferenceDisplay::new(0, 1, (12, 34), (5, 67), true, -11);
    let tdrdi = FrameThreeDReferenceDisplays::new(31, true, 7, vec![tdrdi_display]).unwrap();
    let tdrdi_side_data = FrameSideData::new_three_d_reference_displays(tdrdi.clone()).unwrap();
    assert_eq!(
        tdrdi_side_data.kind_id(),
        &FrameSideDataKind::ThreeDReferenceDisplays
    );
    let parsed_tdrdi = tdrdi_side_data
        .three_d_reference_displays()
        .unwrap()
        .unwrap();
    assert_eq!(parsed_tdrdi, tdrdi);
    assert_eq!(parsed_tdrdi.display(0), Some(tdrdi_display));
    assert_eq!(parsed_tdrdi.display(1), None);
    assert!(tdrdi_display.additional_shift_present());
    assert_eq!(tdrdi_display.num_sample_shift(), -11);
    assert_eq!(tdrdi_side_data.data(), tdrdi.to_bytes());
    for mut data in [
        Vec::new(),
        vec![0; FrameThreeDReferenceDisplays::HEADER_LEN - 1],
        {
            let mut data = tdrdi.to_bytes();
            data[1] = 2;
            data
        },
        {
            let mut data = tdrdi.to_bytes();
            data[3] = 0;
            data
        },
        {
            let mut data = tdrdi.to_bytes();
            data[FrameThreeDReferenceDisplays::ENTRIES_OFFSET + 8] = 2;
            data
        },
    ] {
        assert_eq!(
            FrameThreeDReferenceDisplays::parse(&data)
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
        data.resize(data.len().max(1), 0);
        let invalid_tdrdi =
            FrameSideData::new_with_kind(FrameSideDataKind::ThreeDReferenceDisplays, data).unwrap();
        assert_eq!(
            invalid_tdrdi
                .three_d_reference_displays()
                .unwrap_err()
                .kind(),
            AvErrorKind::InvalidData
        );
    }
    let non_tdrdi =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0]).unwrap();
    assert_eq!(non_tdrdi.three_d_reference_displays().unwrap(), None);

    let exif_bytes = minimal_little_exif_fixture();
    let exif_side_data = FrameSideData::new_exif(exif_bytes.clone()).unwrap();
    assert_eq!(exif_side_data.kind_id(), &FrameSideDataKind::Exif);
    let parsed_exif = exif_side_data.exif().unwrap().unwrap();
    assert_eq!(parsed_exif.data(), exif_bytes.as_slice());
    assert_eq!(parsed_exif.endian(), FrameExifEndian::Little);
    assert_eq!(parsed_exif.first_ifd_offset(), 8);
    assert_eq!(parsed_exif.ifd_count(), 1);
    let exif_ifd = parsed_exif.ifd(0).unwrap();
    assert_eq!(exif_ifd.entry_count(), 1);
    let exif_entry = exif_ifd.entry(0).unwrap();
    assert_eq!(exif_entry.tag(), 0x010F);
    assert_eq!(exif_entry.tiff_type(), FrameExifTiffType::Ascii);
    assert_eq!(exif_entry.count(), 6);
    assert_eq!(exif_entry.data_len(), 6);
    assert_eq!(exif_entry.data_range(), Some((26, 32)));
    assert_eq!(exif_entry.value_data(), b"Rusty\0");
    assert_eq!(exif_entry.ascii_strings().unwrap().unwrap(), ["Rusty"]);
    let mut bad_exif = exif_bytes.clone();
    bad_exif[12..14].copy_from_slice(&0u16.to_le_bytes());
    assert!(exif_payload_invalid(&bad_exif));
    assert_eq!(
        FrameSideData::new_with_kind(FrameSideDataKind::Exif, bad_exif)
            .unwrap()
            .exif()
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_exif = FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0]).unwrap();
    assert_eq!(non_exif.exif().unwrap(), None);

    let typed_exif_side_data = FrameSideData::new_exif(exif_value_semantics_fixture()).unwrap();
    let typed_exif = typed_exif_side_data.exif().unwrap().unwrap();
    let typed_ifd = typed_exif.ifd(0).unwrap();
    assert_eq!(
        typed_ifd
            .entry_by_tag(0x0112)
            .unwrap()
            .short_values()
            .unwrap()
            .unwrap(),
        [6]
    );
    assert_eq!(
        typed_ifd
            .entry_by_tag(0x0100)
            .unwrap()
            .long_values()
            .unwrap()
            .unwrap(),
        [640]
    );
    let typed_rational = typed_ifd
        .entry_by_tag(0x011A)
        .unwrap()
        .rational_values()
        .unwrap()
        .unwrap();
    assert_eq!(typed_rational.len(), 1);
    assert_eq!(typed_rational[0].numerator(), 300);
    assert_eq!(typed_rational[0].denominator(), 1);
    assert_eq!(
        typed_ifd
            .entry_by_tag(0xC001)
            .unwrap()
            .signed_short_values()
            .unwrap()
            .unwrap(),
        [-1, 2]
    );
    let typed_signed_rational = typed_ifd
        .entry_by_tag(0xC003)
        .unwrap()
        .signed_rational_values()
        .unwrap()
        .unwrap();
    assert_eq!(typed_signed_rational.len(), 1);
    assert_eq!(typed_signed_rational[0].numerator(), -1);
    assert_eq!(typed_signed_rational[0].denominator(), 2);
    let mut bad_typed_exif = exif_value_semantics_fixture();
    bad_typed_exif[115] = b'!';
    assert_eq!(
        FrameExif::parse(&bad_typed_exif)
            .unwrap()
            .ifd(0)
            .unwrap()
            .entry_by_tag(0x010F)
            .unwrap()
            .ascii_strings()
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let common_exif_bytes = exif_common_tags_fixture();
    let common_exif = FrameExif::parse(&common_exif_bytes).unwrap();
    let common_tags = common_exif.common_tags().unwrap();
    assert_eq!(common_tags.make(), Some("Rusty"));
    assert_eq!(common_tags.model(), Some("Camera"));
    assert_eq!(common_tags.image_width(), Some(640));
    assert_eq!(common_tags.image_length(), Some(480));
    assert_eq!(
        common_tags.orientation(),
        Some(FrameExifOrientation::RightTop)
    );
    assert_eq!(
        common_tags.resolution_unit(),
        Some(FrameExifResolutionUnit::Inch)
    );
    assert_eq!(common_tags.exif_version(), Some(*b"0231"));
    assert_eq!(
        common_tags.date_time_original(),
        Some("2026:05:04 12:34:56")
    );
    assert_eq!(common_tags.gps_version_id(), Some([2, 3, 0, 0]));
    assert_eq!(
        common_tags.gps_latitude_ref(),
        Some(FrameExifGpsLatitudeRef::North)
    );
    assert_eq!(
        common_tags.gps_longitude_ref(),
        Some(FrameExifGpsLongitudeRef::West)
    );
    assert_eq!(common_tags.interoperability_index(), Some("R98"));

    let gps_altitude_time_exif_bytes = exif_gps_altitude_time_fixture();
    let gps_altitude_time_exif = FrameExif::parse(&gps_altitude_time_exif_bytes).unwrap();
    let gps_altitude_time_tags = gps_altitude_time_exif.common_tags().unwrap();
    assert_eq!(
        gps_altitude_time_tags.gps_altitude_ref(),
        Some(FrameExifGpsAltitudeRef::BelowSeaLevel)
    );
    assert_eq!(gps_altitude_time_tags.gps_altitude_ref().unwrap().raw(), 1);
    assert_eq!(
        gps_altitude_time_tags.gps_altitude().unwrap().numerator(),
        15
    );
    assert_eq!(
        gps_altitude_time_tags.gps_altitude().unwrap().denominator(),
        2
    );
    assert_eq!(
        gps_altitude_time_tags.gps_time_stamp().unwrap()[0].numerator(),
        12
    );
    assert_eq!(
        gps_altitude_time_tags.gps_time_stamp().unwrap()[1].numerator(),
        34
    );
    assert_eq!(
        gps_altitude_time_tags.gps_time_stamp().unwrap()[2].numerator(),
        56
    );
    assert_eq!(gps_altitude_time_tags.gps_date_stamp(), Some("2026:05:06"));

    let gps_acquisition_exif_bytes = exif_gps_acquisition_fixture();
    let gps_acquisition_exif = FrameExif::parse(&gps_acquisition_exif_bytes).unwrap();
    let gps_acquisition_tags = gps_acquisition_exif.common_tags().unwrap();
    assert_eq!(gps_acquisition_tags.gps_satellites(), Some("12 used"));
    assert_eq!(
        gps_acquisition_tags.gps_status(),
        Some(FrameExifGpsStatus::MeasurementInProgress)
    );
    assert_eq!(gps_acquisition_tags.gps_status().unwrap().as_str(), "A");
    assert_eq!(
        gps_acquisition_tags.gps_measure_mode(),
        Some(FrameExifGpsMeasureMode::ThreeDimensional)
    );
    assert_eq!(
        gps_acquisition_tags.gps_measure_mode().unwrap().as_str(),
        "3"
    );
    assert_eq!(gps_acquisition_tags.gps_dop().unwrap().numerator(), 7);
    assert_eq!(gps_acquisition_tags.gps_dop().unwrap().denominator(), 2);

    let gps_motion_exif_bytes = exif_gps_motion_fixture();
    let gps_motion_exif = FrameExif::parse(&gps_motion_exif_bytes).unwrap();
    let gps_motion_tags = gps_motion_exif.common_tags().unwrap();
    assert_eq!(
        gps_motion_tags.gps_speed_ref(),
        Some(FrameExifGpsSpeedRef::KilometersPerHour)
    );
    assert_eq!(gps_motion_tags.gps_speed_ref().unwrap().as_str(), "K");
    assert_eq!(gps_motion_tags.gps_speed().unwrap().numerator(), 88);
    assert_eq!(gps_motion_tags.gps_speed().unwrap().denominator(), 5);
    assert_eq!(
        gps_motion_tags.gps_track_ref(),
        Some(FrameExifGpsDirectionRef::TrueDirection)
    );
    assert_eq!(gps_motion_tags.gps_track_ref().unwrap().as_str(), "T");
    assert_eq!(gps_motion_tags.gps_track().unwrap().numerator(), 270);
    assert_eq!(
        gps_motion_tags.gps_img_direction_ref(),
        Some(FrameExifGpsDirectionRef::MagneticDirection)
    );
    assert_eq!(
        gps_motion_tags.gps_img_direction_ref().unwrap().as_str(),
        "M"
    );
    assert_eq!(
        gps_motion_tags.gps_img_direction().unwrap().numerator(),
        135
    );
    assert_eq!(gps_motion_tags.gps_map_datum(), Some("WGS-84"));

    let gps_destination_exif_bytes = exif_gps_destination_fixture();
    let gps_destination_exif = FrameExif::parse(&gps_destination_exif_bytes).unwrap();
    let gps_destination_tags = gps_destination_exif.common_tags().unwrap();
    assert_eq!(
        gps_destination_tags.gps_dest_latitude_ref(),
        Some(FrameExifGpsLatitudeRef::South)
    );
    assert_eq!(
        gps_destination_tags.gps_dest_latitude().unwrap()[0].numerator(),
        33
    );
    assert_eq!(
        gps_destination_tags.gps_dest_latitude().unwrap()[1].numerator(),
        52
    );
    assert_eq!(
        gps_destination_tags.gps_dest_latitude().unwrap()[2].numerator(),
        7
    );
    assert_eq!(
        gps_destination_tags.gps_dest_longitude_ref(),
        Some(FrameExifGpsLongitudeRef::East)
    );
    assert_eq!(
        gps_destination_tags.gps_dest_longitude().unwrap()[0].numerator(),
        151
    );
    assert_eq!(
        gps_destination_tags.gps_dest_longitude().unwrap()[1].numerator(),
        12
    );
    assert_eq!(
        gps_destination_tags.gps_dest_longitude().unwrap()[2].numerator(),
        9
    );
    assert_eq!(
        gps_destination_tags.gps_dest_bearing_ref(),
        Some(FrameExifGpsDirectionRef::TrueDirection)
    );
    assert_eq!(
        gps_destination_tags
            .gps_dest_bearing_ref()
            .unwrap()
            .as_str(),
        "T"
    );
    assert_eq!(
        gps_destination_tags.gps_dest_bearing().unwrap().numerator(),
        91
    );
    assert_eq!(
        gps_destination_tags
            .gps_dest_bearing()
            .unwrap()
            .denominator(),
        2
    );
    assert_eq!(
        gps_destination_tags.gps_dest_distance_ref(),
        Some(FrameExifGpsDistanceRef::NauticalMiles)
    );
    assert_eq!(
        gps_destination_tags
            .gps_dest_distance_ref()
            .unwrap()
            .as_str(),
        "N"
    );
    assert_eq!(
        gps_destination_tags
            .gps_dest_distance()
            .unwrap()
            .numerator(),
        42
    );

    let exposure_exif_bytes = exif_exposure_tags_fixture();
    let exposure_exif = FrameExif::parse(&exposure_exif_bytes).unwrap();
    let exposure_tags = exposure_exif.common_tags().unwrap();
    assert_eq!(exposure_tags.exposure_time().unwrap().numerator(), 1);
    assert_eq!(exposure_tags.exposure_time().unwrap().denominator(), 125);
    assert_eq!(exposure_tags.f_number().unwrap().numerator(), 28);
    assert_eq!(exposure_tags.f_number().unwrap().denominator(), 10);
    assert_eq!(exposure_tags.exposure_bias_value().unwrap().numerator(), -1);
    assert_eq!(
        exposure_tags.exposure_bias_value().unwrap().denominator(),
        3
    );
    assert_eq!(exposure_tags.focal_length().unwrap().numerator(), 50);
    assert_eq!(exposure_tags.focal_length().unwrap().denominator(), 1);
    assert_eq!(exposure_tags.pixel_x_dimension(), Some(1920));
    assert_eq!(exposure_tags.pixel_y_dimension(), Some(1080));
    assert_eq!(
        exposure_tags.date_time_digitized(),
        Some("2026:05:04 12:35:00")
    );

    let capture_exif_bytes = exif_capture_settings_fixture();
    let capture_exif = FrameExif::parse(&capture_exif_bytes).unwrap();
    let capture_tags = capture_exif.common_tags().unwrap();
    assert_eq!(
        capture_tags.exposure_program(),
        Some(FrameExifExposureProgram::AperturePriority)
    );
    assert_eq!(
        capture_tags.metering_mode(),
        Some(FrameExifMeteringMode::Pattern)
    );
    assert_eq!(capture_tags.light_source(), Some(FrameExifLightSource::D65));
    assert_eq!(capture_tags.flash(), Some(FrameExifFlash::from_raw(0x0041)));
    assert!(capture_tags.flash().unwrap().fired());
    assert!(capture_tags.flash().unwrap().red_eye_reduction_supported());
    assert_eq!(
        capture_tags.white_balance(),
        Some(FrameExifWhiteBalance::Manual)
    );
    assert_eq!(capture_tags.digital_zoom_ratio().unwrap().numerator(), 3);
    assert_eq!(capture_tags.digital_zoom_ratio().unwrap().denominator(), 2);
    assert_eq!(capture_tags.focal_length_in_35mm_film(), Some(75));

    let descriptive_exif_bytes = exif_descriptive_tags_fixture();
    let descriptive_exif = FrameExif::parse(&descriptive_exif_bytes).unwrap();
    let descriptive_tags = descriptive_exif.common_tags().unwrap();
    assert_eq!(descriptive_tags.image_description(), Some("Frame sample"));
    assert_eq!(descriptive_tags.software(), Some("ffmpegrust"));
    assert_eq!(descriptive_tags.date_time(), Some("2026:05:05 01:02:03"));
    assert_eq!(descriptive_tags.artist(), Some("OpenAI"));
    assert_eq!(descriptive_tags.copyright(), Some("2026 Example"));

    let linked_exif_side_data = FrameSideData::new_exif(exif_with_linked_ifds_fixture()).unwrap();
    let linked_exif = linked_exif_side_data.exif().unwrap().unwrap();
    assert_eq!(linked_exif.ifd_count(), 1);
    assert_eq!(linked_exif.linked_ifd_count(), 3);
    let linked_exif_ifd = linked_exif
        .linked_ifd(FrameExifIfdPointerKind::Exif)
        .unwrap();
    assert_eq!(linked_exif_ifd.parent_ifd_offset(), 8);
    assert_eq!(
        linked_exif_ifd.source_tag(),
        FrameExifIfdPointerKind::EXIF_TAG
    );
    assert_eq!(linked_exif_ifd.offset(), 56);
    let linked_gps_ifd = linked_exif
        .linked_ifd(FrameExifIfdPointerKind::Gps)
        .unwrap();
    assert_eq!(linked_gps_ifd.offset(), 74);
    assert_eq!(
        linked_gps_ifd.ifd().entry_by_tag(0).unwrap().value_data(),
        &[2, 3, 0, 0]
    );
    let linked_interop_ifd = linked_exif
        .linked_ifd(FrameExifIfdPointerKind::Interoperability)
        .unwrap();
    assert_eq!(linked_interop_ifd.parent_ifd_offset(), 56);
    assert_eq!(
        linked_interop_ifd
            .ifd()
            .entry_by_tag(1)
            .unwrap()
            .value_data(),
        b"R98\0"
    );
    let mut bad_linked_exif = exif_with_linked_ifds_fixture();
    bad_linked_exif[24..26].copy_from_slice(&FrameExifTiffType::Short.raw().to_le_bytes());
    assert!(exif_payload_invalid(&bad_linked_exif));
    assert_eq!(
        FrameSideData::new_with_kind(FrameSideDataKind::Exif, bad_linked_exif)
            .unwrap()
            .exif()
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );

    let film_grain = minimal_film_grain_av1_fixture();
    let film_grain_side_data = FrameSideData::new_film_grain_params(film_grain.clone()).unwrap();
    let parsed_film_grain = film_grain_side_data.film_grain_params().unwrap().unwrap();
    assert_eq!(
        film_grain_side_data.kind_id(),
        &FrameSideDataKind::FilmGrainParams
    );
    assert_eq!(parsed_film_grain.data(), film_grain.as_slice());
    assert_eq!(
        parsed_film_grain.params_type(),
        FrameFilmGrainParamsType::Av1
    );
    assert_eq!(parsed_film_grain.seed(), 0xAABB_CCDD_EEFF_0011);
    let parsed_aom = parsed_film_grain.aom_params().unwrap().unwrap();
    assert_eq!(parsed_aom.num_y_points(), 1);
    assert_eq!(parsed_aom.y_point(0), Some([24, 9]));
    assert_eq!(parsed_aom.scaling_shift(), 8);
    assert_eq!(parsed_aom.ar_coeff_lag(), 0);
    assert_eq!(
        FrameFilmGrainParams::parse(&[0; FrameFilmGrainParams::DATA_LEN - 1])
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let mut bad_film_grain = film_grain.clone();
    write_ne_i32(
        &mut bad_film_grain,
        film_grain_aom_scaling_shift_offset(),
        7,
    );
    assert_eq!(
        FrameSideData::new_with_kind(FrameSideDataKind::FilmGrainParams, bad_film_grain)
            .unwrap()
            .film_grain_params()
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_film_grain =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0]).unwrap();
    assert_eq!(non_film_grain.film_grain_params().unwrap(), None);

    let detection_bboxes = minimal_detection_bboxes_fixture();
    let detection_side_data =
        FrameSideData::new_detection_bboxes(detection_bboxes.clone()).unwrap();
    let parsed_detection = detection_side_data.detection_bboxes().unwrap().unwrap();
    assert_eq!(
        detection_side_data.kind_id(),
        &FrameSideDataKind::DetectionBboxes
    );
    assert_eq!(parsed_detection.data(), detection_bboxes.as_slice());
    assert_eq!(parsed_detection.source(), b"fuzz-detector");
    assert_eq!(parsed_detection.nb_bboxes(), 1);
    let parsed_bbox = parsed_detection.bbox(0).unwrap().unwrap();
    assert_eq!(parsed_bbox.x(), 1);
    assert_eq!(parsed_bbox.y(), 2);
    assert_eq!(parsed_bbox.width(), 3);
    assert_eq!(parsed_bbox.height(), 4);
    assert_eq!(parsed_bbox.detect_label(), b"object");
    assert_eq!(parsed_bbox.detect_confidence(), Rational::from_raw(1, 2));
    assert_eq!(parsed_bbox.classify_count(), 1);
    assert_eq!(parsed_bbox.classify_label(0), Some(&b"class"[..]));
    assert_eq!(
        parsed_bbox.classify_confidence(0),
        Some(Rational::from_raw(3, 4))
    );
    assert_eq!(
        FrameDetectionBboxes::parse(&[0; FrameDetectionBboxes::HEADER_LEN - 1])
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let mut bad_detection = detection_bboxes.clone();
    write_ne_u32(
        &mut bad_detection,
        detection_bboxes_first_classify_count_offset(),
        FrameDetectionBbox::MAX_CLASSIFICATIONS as u32 + 1,
    );
    assert_eq!(
        FrameSideData::new_with_kind(FrameSideDataKind::DetectionBboxes, bad_detection)
            .unwrap()
            .detection_bboxes()
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_detection =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0]).unwrap();
    assert_eq!(non_detection.detection_bboxes().unwrap(), None);

    let rpu_bytes = vec![0x7C, 0x01, 0x19, 0xAB];
    let rpu_side_data = FrameSideData::new_dolby_vision_rpu_buffer(rpu_bytes.clone()).unwrap();
    let parsed_rpu = rpu_side_data.dolby_vision_rpu_buffer().unwrap();
    assert_eq!(
        rpu_side_data.kind_id(),
        &FrameSideDataKind::DolbyVisionRpuBuffer
    );
    assert_eq!(
        FrameDolbyVisionRpuBuffer::parse(&rpu_bytes).data(),
        rpu_bytes.as_slice()
    );
    assert_eq!(parsed_rpu.data(), rpu_bytes.as_slice());
    assert!(!parsed_rpu.is_empty());

    let dovi_metadata = minimal_dolby_vision_metadata_fixture();
    let dovi_side_data = FrameSideData::new_dolby_vision_metadata(dovi_metadata.clone()).unwrap();
    let parsed_dovi = dovi_side_data.dolby_vision_metadata().unwrap().unwrap();
    assert_eq!(
        dovi_side_data.kind_id(),
        &FrameSideDataKind::DolbyVisionMetadata
    );
    assert_eq!(parsed_dovi.data(), dovi_metadata.as_slice());
    assert_eq!(parsed_dovi.num_ext_blocks(), 2);
    let dovi_header = parsed_dovi.header().unwrap();
    assert_eq!(
        dovi_header.data().len(),
        FrameDolbyVisionRpuDataHeader::DATA_LEN
    );
    assert_eq!(dovi_header.rpu_type(), 2);
    assert_eq!(dovi_header.rpu_format(), 18);
    assert_eq!(dovi_header.vdr_rpu_profile(), 8);
    assert_eq!(dovi_header.bl_bit_depth(), 10);
    let dovi_mapping = parsed_dovi.mapping().unwrap();
    assert_eq!(
        dovi_mapping.data().len(),
        FrameDolbyVisionDataMapping::DATA_LEN
    );
    assert_eq!(dovi_mapping.vdr_rpu_id(), 3);
    assert_eq!(dovi_mapping.num_x_partitions(), 1);
    let dovi_color = parsed_dovi.color().unwrap();
    assert_eq!(
        dovi_color.data().len(),
        FrameDolbyVisionColorMetadata::DATA_LEN
    );
    assert_eq!(dovi_color.dm_metadata_id(), 9);
    assert_eq!(
        dovi_color.ycc_to_rgb_matrix(0),
        Some(Rational::from_raw(1, 2))
    );
    assert_eq!(dovi_color.signal_eotf(), 2084);
    assert_eq!(dovi_color.signal_full_range_flag(), 3);
    let dovi_level1 = parsed_dovi.ext_block(0).unwrap().unwrap();
    assert_eq!(dovi_level1.data().len(), FrameDolbyVisionDmData::DATA_LEN);
    assert_eq!(dovi_level1.level(), 1);
    assert_eq!(dovi_level1.level1_avg_pq(), Some(512));
    let dovi_level6 = parsed_dovi.find_level(6).unwrap().unwrap();
    assert_eq!(dovi_level6.level6_max_content_light_level(), Some(800));
    assert_eq!(
        FrameDolbyVisionMetadata::parse(&[0; FrameDolbyVisionMetadata::DATA_LEN - 1])
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let mut bad_dovi = dovi_metadata.clone();
    write_ne_i32(
        &mut bad_dovi,
        dovi_num_ext_blocks_field_offset(),
        FrameDolbyVisionMetadata::MAX_EXT_BLOCKS as i32 + 1,
    );
    assert_eq!(
        FrameSideData::new_with_kind(FrameSideDataKind::DolbyVisionMetadata, bad_dovi)
            .unwrap()
            .dolby_vision_metadata()
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_dovi = FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0]).unwrap();
    assert_eq!(non_dovi.dolby_vision_rpu_buffer(), None);
    assert_eq!(non_dovi.dolby_vision_metadata().unwrap(), None);

    let vivid_metadata = minimal_dynamic_hdr_vivid_fixture();
    let vivid_side_data = FrameSideData::new_dynamic_hdr_vivid(vivid_metadata.clone()).unwrap();
    let parsed_vivid = vivid_side_data.dynamic_hdr_vivid().unwrap().unwrap();
    assert_eq!(
        vivid_side_data.kind_id(),
        &FrameSideDataKind::DynamicHdrVivid
    );
    assert_eq!(parsed_vivid.data(), vivid_metadata.as_slice());
    assert_eq!(
        parsed_vivid.system_start_code(),
        FrameDynamicHdrVivid::MIN_SYSTEM_START_CODE
    );
    assert_eq!(parsed_vivid.num_windows(), 1);
    let vivid_params = parsed_vivid.color_transform_params(0).unwrap();
    assert_eq!(vivid_params.minimum_maxrgb(), Rational::from_raw(1, 4095));
    assert_eq!(vivid_params.tone_mapping_mode_flag(), 1);
    assert_eq!(vivid_params.tone_mapping_param_num(), 1);
    let vivid_tm = vivid_params.tone_mapping_params(0).unwrap();
    assert_eq!(vivid_tm.base_enable_flag(), 1);
    assert_eq!(vivid_tm.three_spline_enable_flag(), 1);
    let vivid_spline = vivid_tm.three_spline(0).unwrap().unwrap();
    assert_eq!(vivid_spline.th_mode(), 0);
    assert_eq!(vivid_params.color_saturation_mapping_flag(), 1);
    assert_eq!(
        vivid_params.color_saturation_gain(0),
        Some(Rational::from_raw(1, 128))
    );
    assert_eq!(
        FrameDynamicHdrVivid::parse(&[0; FrameDynamicHdrVivid::DATA_LEN - 1])
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let mut bad_vivid = vivid_metadata.clone();
    write_ne_i32(&mut bad_vivid, hdr_vivid_tone_mapping_mode_flag_offset(), 2);
    assert_eq!(
        FrameSideData::new_with_kind(FrameSideDataKind::DynamicHdrVivid, bad_vivid)
            .unwrap()
            .dynamic_hdr_vivid()
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_vivid =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0]).unwrap();
    assert_eq!(non_vivid.dynamic_hdr_vivid().unwrap(), None);

    let ambient = FrameAmbientViewingEnvironment::new(
        Rational::from_raw(203, 10),
        Rational::from_raw(15_635, 50_000),
        Rational::from_raw(16_450, 50_000),
    )
    .unwrap();
    let ambient_side_data = FrameSideData::new_ambient_viewing_environment(ambient).unwrap();
    assert_eq!(
        ambient_side_data.kind_id(),
        &FrameSideDataKind::AmbientViewingEnvironment
    );
    assert_eq!(
        ambient_side_data.ambient_viewing_environment().unwrap(),
        Some(ambient)
    );
    assert_eq!(
        FrameAmbientViewingEnvironment::parse(&ambient.to_bytes()).unwrap(),
        ambient
    );
    let mut bad_ambient = ambient.to_bytes();
    write_ne_rational(
        &mut bad_ambient,
        FrameAmbientViewingEnvironment::AMBIENT_LIGHT_X_OFFSET,
        Rational::from_raw(2, 1),
    );
    assert_eq!(
        FrameAmbientViewingEnvironment::parse(&bad_ambient)
            .unwrap_err()
            .kind(),
        AvErrorKind::InvalidData
    );
    let non_ambient =
        FrameSideData::new_with_kind(FrameSideDataKind::DisplayMatrix, vec![0]).unwrap();
    assert_eq!(non_ambient.ambient_viewing_environment().unwrap(), None);

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
    match byte.unwrap_or_default() % 33 {
        0 => FrameSideDataKind::PanScan,
        1 => FrameSideDataKind::A53ClosedCaptions,
        2 => FrameSideDataKind::Stereo3d,
        3 => FrameSideDataKind::DisplayMatrix,
        4 => FrameSideDataKind::MatrixEncoding,
        5 => FrameSideDataKind::DownmixInfo,
        6 => FrameSideDataKind::ReplayGain,
        7 => FrameSideDataKind::MotionVectors,
        8 => FrameSideDataKind::MasteringDisplayMetadata,
        9 => FrameSideDataKind::Spherical,
        10 => FrameSideDataKind::ContentLightLevel,
        11 => FrameSideDataKind::IccProfile,
        12 => FrameSideDataKind::DolbyVisionRpuBuffer,
        13 => FrameSideDataKind::Lcevc,
        14 => FrameSideDataKind::GopTimecode,
        15 => FrameSideDataKind::S12mTimecode,
        16 => FrameSideDataKind::DynamicHdrPlus,
        17 => FrameSideDataKind::RegionsOfInterest,
        18 => FrameSideDataKind::VideoEncParams,
        19 => FrameSideDataKind::VideoHint,
        20 => FrameSideDataKind::ViewId,
        21 => FrameSideDataKind::ThreeDReferenceDisplays,
        22 => FrameSideDataKind::Exif,
        23 => FrameSideDataKind::SeiUnregistered,
        24 => FrameSideDataKind::ActiveFormatDescription,
        25 => FrameSideDataKind::SkipSamples,
        26 => FrameSideDataKind::AudioServiceType,
        27 => FrameSideDataKind::FilmGrainParams,
        28 => FrameSideDataKind::DetectionBboxes,
        29 => FrameSideDataKind::DolbyVisionMetadata,
        30 => FrameSideDataKind::DynamicHdrVivid,
        31 => FrameSideDataKind::AmbientViewingEnvironment,
        _ => FrameSideDataKind::Unknown(String::from("fuzz_frame_side_data")),
    }
}

fn minimal_dynamic_hdr_plus_fixture() -> Vec<u8> {
    let mut data = vec![0; FrameDynamicHdrPlus::DATA_LEN];
    data[0] = FrameDynamicHdrPlus::ITU_T_T35_COUNTRY_CODE;
    data[1] = FrameDynamicHdrPlus::APPLICATION_VERSION;
    data[2] = 1;
    write_ne_i32(
        &mut data,
        4 + 44,
        FrameHdrPlusOverlapProcessOption::WeightedAveraging.as_raw(),
    );
    data
}

fn minimal_dynamic_hdr_vivid_fixture() -> Vec<u8> {
    let mut data = vec![0; FrameDynamicHdrVivid::DATA_LEN];
    data[0] = FrameDynamicHdrVivid::MIN_SYSTEM_START_CODE;
    data[1] = 1;

    let params = hdr_vivid_params_offset(0);
    write_ne_rational(&mut data, params, Rational::from_raw(1, 4095));
    write_ne_i32(&mut data, hdr_vivid_tone_mapping_mode_flag_offset(), 1);
    write_ne_i32(&mut data, hdr_vivid_tone_mapping_param_num_offset(), 1);

    let tm = hdr_vivid_tone_mapping_params_offset();
    write_ne_rational(&mut data, tm, Rational::from_raw(100, 4095));
    write_ne_i32(&mut data, tm + 8, 1);
    write_ne_i32(&mut data, tm + 76, 1);
    write_ne_i32(&mut data, tm + 80, 1);
    write_ne_i32(&mut data, tm + 84, 0);

    write_ne_i32(&mut data, hdr_vivid_color_saturation_flag_offset(), 1);
    write_ne_i32(&mut data, hdr_vivid_color_saturation_num_offset(), 1);
    write_ne_rational(
        &mut data,
        hdr_vivid_color_saturation_gain_offset(),
        Rational::from_raw(1, 128),
    );
    data
}

fn pan_scan_payload_invalid(data: &[u8]) -> bool {
    data.len() != FramePanScan::DATA_LEN
}

fn a53_closed_captions_payload_invalid(data: &[u8]) -> bool {
    !data
        .len()
        .is_multiple_of(FrameA53ClosedCaptions::BYTES_PER_CC)
}

fn stereo3d_payload_invalid(data: &[u8]) -> bool {
    if data.len() != FrameStereo3d::DATA_LEN {
        return true;
    }
    if FrameStereo3dType::from_raw(read_ne_i32(data, FrameStereo3d::TYPE_OFFSET)).is_err() {
        return true;
    }
    if FrameStereo3dFlags::from_raw(read_ne_i32(data, FrameStereo3d::FLAGS_OFFSET)).is_err() {
        return true;
    }
    if FrameStereo3dView::from_raw(read_ne_i32(data, FrameStereo3d::VIEW_OFFSET)).is_err() {
        return true;
    }
    if FrameStereo3dPrimaryEye::from_raw(read_ne_i32(data, FrameStereo3d::PRIMARY_EYE_OFFSET))
        .is_err()
    {
        return true;
    }

    !stereo3d_disparity_valid(read_ne_rational(
        data,
        FrameStereo3d::HORIZONTAL_DISPARITY_ADJUSTMENT_OFFSET,
    )) || !stereo3d_field_of_view_valid(read_ne_rational(
        data,
        FrameStereo3d::HORIZONTAL_FIELD_OF_VIEW_OFFSET,
    ))
}

fn stereo3d_disparity_valid(value: Rational) -> bool {
    stereo3d_rational_set_or_zero(value)
        && (value.den() == 0 || i64::from(value.num()).abs() <= i64::from(value.den()).abs())
}

fn stereo3d_field_of_view_valid(value: Rational) -> bool {
    stereo3d_rational_set_or_zero(value)
        && (value.den() == 0
            || value.num() == 0
            || value.num().is_positive() == value.den().is_positive())
}

fn stereo3d_rational_set_or_zero(value: Rational) -> bool {
    value.den() != 0 || value.num() == 0
}

fn ambient_viewing_environment_payload_invalid(data: &[u8]) -> bool {
    if data.len() != FrameAmbientViewingEnvironment::DATA_LEN {
        return true;
    }

    !ambient_nonnegative_rational(read_ne_rational(
        data,
        FrameAmbientViewingEnvironment::AMBIENT_ILLUMINANCE_OFFSET,
    )) || !ambient_unit_interval_rational(read_ne_rational(
        data,
        FrameAmbientViewingEnvironment::AMBIENT_LIGHT_X_OFFSET,
    )) || !ambient_unit_interval_rational(read_ne_rational(
        data,
        FrameAmbientViewingEnvironment::AMBIENT_LIGHT_Y_OFFSET,
    ))
}

fn ambient_nonnegative_rational(value: Rational) -> bool {
    value.den() != 0 && (value.num() == 0 || value.num().is_positive() == value.den().is_positive())
}

fn ambient_unit_interval_rational(value: Rational) -> bool {
    ambient_nonnegative_rational(value)
        && i64::from(value.num()).abs() <= i64::from(value.den()).abs()
}

fn dynamic_hdr_plus_payload_invalid(data: &[u8]) -> bool {
    if data.len() != FrameDynamicHdrPlus::DATA_LEN {
        return true;
    }
    if data[0] != FrameDynamicHdrPlus::ITU_T_T35_COUNTRY_CODE {
        return true;
    }
    if data[1] != FrameDynamicHdrPlus::APPLICATION_VERSION {
        return true;
    }

    let num_windows = usize::from(data[2]);
    if !(1..=FrameDynamicHdrPlus::MAX_WINDOWS).contains(&num_windows) {
        return true;
    }

    const PARAMS_OFFSET: usize = 4;
    const OVERLAP_PROCESS_OPTION_OFFSET: usize = 44;
    const NUM_DISTRIBUTION_MAXRGB_PERCENTILES_OFFSET: usize = 80;
    const TONE_MAPPING_FLAG_OFFSET: usize = 272;
    const NUM_BEZIER_CURVE_ANCHORS_OFFSET: usize = 292;
    const COLOR_SATURATION_MAPPING_FLAG_OFFSET: usize = 416;
    for window in 0..num_windows {
        let offset = PARAMS_OFFSET + window * FrameHdrPlusColorTransformParams::DATA_LEN;
        if !matches!(
            read_ne_i32(data, offset + OVERLAP_PROCESS_OPTION_OFFSET),
            0 | 1
        ) {
            return true;
        }
        if usize::from(data[offset + NUM_DISTRIBUTION_MAXRGB_PERCENTILES_OFFSET])
            > FrameHdrPlusColorTransformParams::MAX_DISTRIBUTION_MAXRGB_PERCENTILES
        {
            return true;
        }
        if data[offset + TONE_MAPPING_FLAG_OFFSET] > 1 {
            return true;
        }
        if usize::from(data[offset + NUM_BEZIER_CURVE_ANCHORS_OFFSET])
            > FrameHdrPlusColorTransformParams::MAX_BEZIER_CURVE_ANCHORS
        {
            return true;
        }
        if data[offset + COLOR_SATURATION_MAPPING_FLAG_OFFSET] > 1 {
            return true;
        }
    }

    let peak_luminance_table_len = 25 * 25 * 8;
    let target_max_luminance_offset = PARAMS_OFFSET
        + FrameDynamicHdrPlus::MAX_WINDOWS * FrameHdrPlusColorTransformParams::DATA_LEN;
    let target_flag_offset = target_max_luminance_offset + 8;
    let target_rows_offset = target_flag_offset + 1;
    let target_cols_offset = target_rows_offset + 1;
    if peak_luminance_grid_invalid(
        data[target_flag_offset],
        data[target_rows_offset],
        data[target_cols_offset],
    ) {
        return true;
    }

    let target_table_offset = target_max_luminance_offset + 12;
    let mastering_flag_offset = target_table_offset + peak_luminance_table_len;
    let mastering_rows_offset = mastering_flag_offset + 1;
    let mastering_cols_offset = mastering_rows_offset + 1;
    peak_luminance_grid_invalid(
        data[mastering_flag_offset],
        data[mastering_rows_offset],
        data[mastering_cols_offset],
    )
}

fn peak_luminance_grid_invalid(flag: u8, rows: u8, cols: u8) -> bool {
    if flag > 1 {
        return true;
    }
    if flag == 1 && (!(2..=25).contains(&rows) || !(2..=25).contains(&cols)) {
        return true;
    }
    false
}

fn dynamic_hdr_vivid_payload_invalid(data: &[u8]) -> bool {
    if data.len() != FrameDynamicHdrVivid::DATA_LEN {
        return true;
    }
    if !(FrameDynamicHdrVivid::MIN_SYSTEM_START_CODE..=FrameDynamicHdrVivid::MAX_SYSTEM_START_CODE)
        .contains(&data[0])
    {
        return true;
    }

    let num_windows = usize::from(data[1]);
    if !(1..=FrameDynamicHdrVivid::MAX_WINDOWS).contains(&num_windows) {
        return true;
    }

    for window in 0..num_windows {
        let params = hdr_vivid_params_offset(window);
        if !matches!(read_ne_i32(data, params + 32), 0 | 1) {
            return true;
        }
        if read_ne_i32(data, params + 32) == 1 {
            let tone_mapping_count = read_ne_i32(data, params + 36);
            if !(1..=FrameHdrVividColorTransformParams::MAX_TONE_MAPPING_PARAMS as i32)
                .contains(&tone_mapping_count)
            {
                return true;
            }
            for tm_index in 0..tone_mapping_count as usize {
                let tm = params + 40 + tm_index * FrameHdrVividColorToneMappingParams::DATA_LEN;
                if !matches!(read_ne_i32(data, tm + 8), 0 | 1) {
                    return true;
                }
                if !matches!(read_ne_i32(data, tm + 76), 0 | 1) {
                    return true;
                }
                if read_ne_i32(data, tm + 76) == 1 {
                    let spline_count = read_ne_i32(data, tm + 80);
                    if !(1..=FrameHdrVividColorToneMappingParams::MAX_THREE_SPLINES as i32)
                        .contains(&spline_count)
                    {
                        return true;
                    }
                    for spline_index in 0..spline_count as usize {
                        let spline = tm + 84 + spline_index * FrameHdrVivid3SplineParams::DATA_LEN;
                        if !(0..=3).contains(&read_ne_i32(data, spline)) {
                            return true;
                        }
                    }
                }
            }
        }

        if !matches!(read_ne_i32(data, params + 384), 0 | 1) {
            return true;
        }
        if read_ne_i32(data, params + 384) == 1 {
            let saturation_count = read_ne_i32(data, params + 388);
            if !(0..=FrameHdrVividColorTransformParams::MAX_COLOR_SATURATION_GAINS as i32)
                .contains(&saturation_count)
            {
                return true;
            }
        }
    }

    false
}

fn hdr_vivid_params_offset(window: usize) -> usize {
    4 + window * FrameHdrVividColorTransformParams::DATA_LEN
}

fn hdr_vivid_tone_mapping_mode_flag_offset() -> usize {
    hdr_vivid_params_offset(0) + 32
}

fn hdr_vivid_tone_mapping_param_num_offset() -> usize {
    hdr_vivid_params_offset(0) + 36
}

fn hdr_vivid_tone_mapping_params_offset() -> usize {
    hdr_vivid_params_offset(0) + 40
}

fn hdr_vivid_color_saturation_flag_offset() -> usize {
    hdr_vivid_params_offset(0) + 384
}

fn hdr_vivid_color_saturation_num_offset() -> usize {
    hdr_vivid_params_offset(0) + 388
}

fn hdr_vivid_color_saturation_gain_offset() -> usize {
    hdr_vivid_params_offset(0) + 392
}

fn regions_of_interest_payload_invalid(data: &[u8]) -> bool {
    if data.is_empty()
        || !data
            .chunks_exact(FrameRegionOfInterest::DATA_LEN)
            .remainder()
            .is_empty()
    {
        return true;
    }

    for region in data.chunks_exact(FrameRegionOfInterest::DATA_LEN) {
        if read_ne_u32(region, 0) != FrameRegionOfInterest::SELF_SIZE {
            return true;
        }
        if !region_qoffset_valid(Rational::from_raw(
            read_ne_i32(region, 20),
            read_ne_i32(region, 24),
        )) {
            return true;
        }
    }

    false
}

fn region_qoffset_valid(qoffset: Rational) -> bool {
    if qoffset.den() == 0 {
        return false;
    }

    let mut num = i128::from(qoffset.num());
    let mut den = i128::from(qoffset.den());
    if den < 0 {
        num = -num;
        den = -den;
    }

    (-den..=den).contains(&num)
}

fn video_enc_params_payload_invalid(data: &[u8]) -> bool {
    if data.len() < FrameVideoEncParams::HEADER_LEN {
        return true;
    }

    if !matches!(
        read_ne_i32(data, video_enc_params_type_field_offset()),
        -1..=2
    ) {
        return true;
    }

    if read_ne_usize(data, video_enc_params_blocks_offset_field_offset())
        != FrameVideoEncParams::HEADER_LEN
    {
        return true;
    }

    if read_ne_usize(data, video_enc_params_block_size_field_offset())
        != FrameVideoBlockParams::DATA_LEN
    {
        return true;
    }

    let nb_blocks = read_ne_u32(data, 0) as usize;
    let Some(blocks_len) = nb_blocks.checked_mul(FrameVideoBlockParams::DATA_LEN) else {
        return true;
    };
    let Some(expected_len) = FrameVideoEncParams::HEADER_LEN.checked_add(blocks_len) else {
        return true;
    };
    if data.len() != expected_len {
        return true;
    }

    for block in
        data[FrameVideoEncParams::HEADER_LEN..].chunks_exact(FrameVideoBlockParams::DATA_LEN)
    {
        if read_ne_i32(block, 8) <= 0 || read_ne_i32(block, 12) <= 0 {
            return true;
        }
    }

    false
}

fn video_enc_params_blocks_offset_field_offset() -> usize {
    if core::mem::size_of::<usize>() == 8 {
        8
    } else {
        4
    }
}

fn video_enc_params_block_size_field_offset() -> usize {
    video_enc_params_blocks_offset_field_offset() + core::mem::size_of::<usize>()
}

fn video_enc_params_type_field_offset() -> usize {
    video_enc_params_block_size_field_offset() + core::mem::size_of::<usize>()
}

fn video_hint_payload_invalid(data: &[u8]) -> bool {
    if data.len() < FrameVideoHint::HEADER_LEN {
        return true;
    }

    if read_ne_usize(data, video_hint_rect_offset_field_offset()) != FrameVideoHint::HEADER_LEN {
        return true;
    }

    if read_ne_usize(data, video_hint_rect_size_field_offset()) != FrameVideoRect::DATA_LEN {
        return true;
    }

    if !matches!(read_ne_i32(data, video_hint_type_field_offset()), 0 | 1) {
        return true;
    }

    let nb_rects = read_ne_usize(data, 0);
    let Some(rects_len) = nb_rects.checked_mul(FrameVideoRect::DATA_LEN) else {
        return true;
    };
    let Some(expected_len) = FrameVideoHint::HEADER_LEN.checked_add(rects_len) else {
        return true;
    };
    if data.len() != expected_len {
        return true;
    }

    for rect in data[FrameVideoHint::HEADER_LEN..].chunks_exact(FrameVideoRect::DATA_LEN) {
        let x = read_ne_u32(rect, 0);
        let y = read_ne_u32(rect, 4);
        let width = read_ne_u32(rect, 8);
        let height = read_ne_u32(rect, 12);
        if width == 0
            || height == 0
            || x.checked_add(width).is_none()
            || y.checked_add(height).is_none()
        {
            return true;
        }
    }

    false
}

fn video_hint_rect_offset_field_offset() -> usize {
    core::mem::size_of::<usize>()
}

fn video_hint_rect_size_field_offset() -> usize {
    video_hint_rect_offset_field_offset() + core::mem::size_of::<usize>()
}

fn video_hint_type_field_offset() -> usize {
    video_hint_rect_size_field_offset() + core::mem::size_of::<usize>()
}

fn three_d_reference_displays_payload_invalid(data: &[u8]) -> bool {
    if data.len() < FrameThreeDReferenceDisplays::HEADER_LEN {
        return true;
    }

    if data[0] > 31 || !matches!(data[1], 0 | 1) || data[2] > 31 {
        return true;
    }

    let nb_displays = data[3] as usize;
    if !(1..=FrameThreeDReferenceDisplays::MAX_REF_DISPLAYS).contains(&nb_displays) {
        return true;
    }

    if read_ne_usize(data, FrameThreeDReferenceDisplays::ENTRIES_OFFSET_OFFSET)
        != FrameThreeDReferenceDisplays::ENTRIES_OFFSET
    {
        return true;
    }

    if read_ne_usize(data, FrameThreeDReferenceDisplays::ENTRY_SIZE_OFFSET)
        != FrameThreeDReferenceDisplay::DATA_LEN
    {
        return true;
    }

    let Some(displays_len) = nb_displays.checked_mul(FrameThreeDReferenceDisplay::DATA_LEN) else {
        return true;
    };
    let Some(expected_len) = FrameThreeDReferenceDisplays::ENTRIES_OFFSET.checked_add(displays_len)
    else {
        return true;
    };
    if data.len() != expected_len {
        return true;
    }

    for display in data[FrameThreeDReferenceDisplays::ENTRIES_OFFSET..]
        .chunks_exact(FrameThreeDReferenceDisplay::DATA_LEN)
    {
        if !matches!(display[8], 0 | 1) {
            return true;
        }
    }

    false
}

fn minimal_little_exif_fixture() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
    data.extend_from_slice(&8u32.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&0x010Fu16.to_le_bytes());
    data.extend_from_slice(&FrameExifTiffType::Ascii.raw().to_le_bytes());
    data.extend_from_slice(&6u32.to_le_bytes());
    data.extend_from_slice(&26u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(b"Rusty\0");
    data
}

fn exif_value_semantics_fixture() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
    data.extend_from_slice(&8u32.to_le_bytes());
    data.extend_from_slice(&8u16.to_le_bytes());
    push_exif_entry(
        &mut data,
        0x010F,
        FrameExifTiffType::Ascii,
        6,
        110u32.to_le_bytes(),
    );
    push_exif_entry(&mut data, 0x0112, FrameExifTiffType::Short, 1, [6, 0, 0, 0]);
    push_exif_entry(
        &mut data,
        0x0100,
        FrameExifTiffType::Long,
        1,
        640u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        0x011A,
        FrameExifTiffType::Rational,
        1,
        116u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        0xC001,
        FrameExifTiffType::SignedShort,
        2,
        [0xFF, 0xFF, 0x02, 0x00],
    );
    push_exif_entry(
        &mut data,
        0xC002,
        FrameExifTiffType::SignedLong,
        1,
        (-42i32).to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        0xC003,
        FrameExifTiffType::SignedRational,
        1,
        124u32.to_le_bytes(),
    );
    push_exif_entry(&mut data, 0, FrameExifTiffType::Byte, 4, [2, 3, 0, 0]);
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(b"Rusty\0");
    data.extend_from_slice(&300u32.to_le_bytes());
    data.extend_from_slice(&1u32.to_le_bytes());
    data.extend_from_slice(&(-1i32).to_le_bytes());
    data.extend_from_slice(&2i32.to_le_bytes());
    data
}

fn exif_common_tags_fixture() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
    data.extend_from_slice(&8u32.to_le_bytes());
    data.extend_from_slice(&9u16.to_le_bytes());
    push_exif_entry(
        &mut data,
        FrameExif::TAG_MAKE,
        FrameExifTiffType::Ascii,
        6,
        122u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_MODEL,
        FrameExifTiffType::Ascii,
        7,
        128u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_IMAGE_WIDTH,
        FrameExifTiffType::Long,
        1,
        640u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_IMAGE_LENGTH,
        FrameExifTiffType::Short,
        1,
        [0xE0, 0x01, 0, 0],
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_ORIENTATION,
        FrameExifTiffType::Short,
        1,
        [6, 0, 0, 0],
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_X_RESOLUTION,
        FrameExifTiffType::Rational,
        1,
        136u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_RESOLUTION_UNIT,
        FrameExifTiffType::Short,
        1,
        [2, 0, 0, 0],
    );
    push_exif_entry(
        &mut data,
        FrameExifIfdPointerKind::EXIF_TAG,
        FrameExifTiffType::Long,
        1,
        144u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExifIfdPointerKind::GPS_TAG,
        FrameExifTiffType::Long,
        1,
        224u32.to_le_bytes(),
    );
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(b"Rusty\0");
    data.extend_from_slice(b"Camera\0");
    data.push(0);
    data.extend_from_slice(&300u32.to_le_bytes());
    data.extend_from_slice(&1u32.to_le_bytes());

    data.extend_from_slice(&3u16.to_le_bytes());
    push_exif_entry(
        &mut data,
        FrameExif::TAG_EXIF_VERSION,
        FrameExifTiffType::Undefined,
        4,
        *b"0231",
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_DATE_TIME_ORIGINAL,
        FrameExifTiffType::Ascii,
        20,
        186u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExifIfdPointerKind::INTEROPERABILITY_TAG,
        FrameExifTiffType::Long,
        1,
        206u32.to_le_bytes(),
    );
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(b"2026:05:04 12:34:56\0");

    data.extend_from_slice(&1u16.to_le_bytes());
    push_exif_entry(
        &mut data,
        FrameExif::TAG_INTEROPERABILITY_INDEX,
        FrameExifTiffType::Ascii,
        4,
        *b"R98\0",
    );
    data.extend_from_slice(&0u32.to_le_bytes());

    data.extend_from_slice(&5u16.to_le_bytes());
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_VERSION_ID,
        FrameExifTiffType::Byte,
        4,
        [2, 3, 0, 0],
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_LATITUDE_REF,
        FrameExifTiffType::Ascii,
        2,
        [b'N', 0, 0, 0],
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_LATITUDE,
        FrameExifTiffType::Rational,
        3,
        290u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_LONGITUDE_REF,
        FrameExifTiffType::Ascii,
        2,
        [b'W', 0, 0, 0],
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_LONGITUDE,
        FrameExifTiffType::Rational,
        3,
        314u32.to_le_bytes(),
    );
    data.extend_from_slice(&0u32.to_le_bytes());
    for value in [37u32, 48, 30, 122, 24, 15] {
        data.extend_from_slice(&value.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
    }
    data
}

fn exif_gps_altitude_time_fixture() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
    data.extend_from_slice(&8u32.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    push_exif_entry(
        &mut data,
        FrameExifIfdPointerKind::GPS_TAG,
        FrameExifTiffType::Long,
        1,
        26u32.to_le_bytes(),
    );
    data.extend_from_slice(&0u32.to_le_bytes());

    data.extend_from_slice(&4u16.to_le_bytes());
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_ALTITUDE_REF,
        FrameExifTiffType::Byte,
        1,
        [1, 0, 0, 0],
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_ALTITUDE,
        FrameExifTiffType::Rational,
        1,
        80u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_TIME_STAMP,
        FrameExifTiffType::Rational,
        3,
        88u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_DATE_STAMP,
        FrameExifTiffType::Ascii,
        11,
        112u32.to_le_bytes(),
    );
    data.extend_from_slice(&0u32.to_le_bytes());

    data.extend_from_slice(&15u32.to_le_bytes());
    data.extend_from_slice(&2u32.to_le_bytes());
    for value in [12u32, 34, 56] {
        data.extend_from_slice(&value.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
    }
    data.extend_from_slice(b"2026:05:06\0");
    data
}

fn exif_gps_acquisition_fixture() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
    data.extend_from_slice(&8u32.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    push_exif_entry(
        &mut data,
        FrameExifIfdPointerKind::GPS_TAG,
        FrameExifTiffType::Long,
        1,
        26u32.to_le_bytes(),
    );
    data.extend_from_slice(&0u32.to_le_bytes());

    data.extend_from_slice(&4u16.to_le_bytes());
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_SATELLITES,
        FrameExifTiffType::Ascii,
        8,
        80u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_STATUS,
        FrameExifTiffType::Ascii,
        2,
        [b'A', 0, 0, 0],
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_MEASURE_MODE,
        FrameExifTiffType::Ascii,
        2,
        [b'3', 0, 0, 0],
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_DOP,
        FrameExifTiffType::Rational,
        1,
        88u32.to_le_bytes(),
    );
    data.extend_from_slice(&0u32.to_le_bytes());

    data.extend_from_slice(b"12 used\0");
    data.extend_from_slice(&7u32.to_le_bytes());
    data.extend_from_slice(&2u32.to_le_bytes());
    data
}

fn exif_gps_motion_fixture() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
    data.extend_from_slice(&8u32.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    push_exif_entry(
        &mut data,
        FrameExifIfdPointerKind::GPS_TAG,
        FrameExifTiffType::Long,
        1,
        26u32.to_le_bytes(),
    );
    data.extend_from_slice(&0u32.to_le_bytes());

    data.extend_from_slice(&7u16.to_le_bytes());
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_SPEED_REF,
        FrameExifTiffType::Ascii,
        2,
        [b'K', 0, 0, 0],
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_SPEED,
        FrameExifTiffType::Rational,
        1,
        116u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_TRACK_REF,
        FrameExifTiffType::Ascii,
        2,
        [b'T', 0, 0, 0],
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_TRACK,
        FrameExifTiffType::Rational,
        1,
        124u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_IMG_DIRECTION_REF,
        FrameExifTiffType::Ascii,
        2,
        [b'M', 0, 0, 0],
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_IMG_DIRECTION,
        FrameExifTiffType::Rational,
        1,
        132u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_MAP_DATUM,
        FrameExifTiffType::Ascii,
        7,
        140u32.to_le_bytes(),
    );
    data.extend_from_slice(&0u32.to_le_bytes());

    data.extend_from_slice(&88u32.to_le_bytes());
    data.extend_from_slice(&5u32.to_le_bytes());
    data.extend_from_slice(&270u32.to_le_bytes());
    data.extend_from_slice(&1u32.to_le_bytes());
    data.extend_from_slice(&135u32.to_le_bytes());
    data.extend_from_slice(&1u32.to_le_bytes());
    data.extend_from_slice(b"WGS-84\0");
    data
}

fn exif_gps_destination_fixture() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
    data.extend_from_slice(&8u32.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    push_exif_entry(
        &mut data,
        FrameExifIfdPointerKind::GPS_TAG,
        FrameExifTiffType::Long,
        1,
        26u32.to_le_bytes(),
    );
    data.extend_from_slice(&0u32.to_le_bytes());

    data.extend_from_slice(&8u16.to_le_bytes());
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_DEST_LATITUDE_REF,
        FrameExifTiffType::Ascii,
        2,
        [b'S', 0, 0, 0],
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_DEST_LATITUDE,
        FrameExifTiffType::Rational,
        3,
        128u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_DEST_LONGITUDE_REF,
        FrameExifTiffType::Ascii,
        2,
        [b'E', 0, 0, 0],
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_DEST_LONGITUDE,
        FrameExifTiffType::Rational,
        3,
        152u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_DEST_BEARING_REF,
        FrameExifTiffType::Ascii,
        2,
        [b'T', 0, 0, 0],
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_DEST_BEARING,
        FrameExifTiffType::Rational,
        1,
        176u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_DEST_DISTANCE_REF,
        FrameExifTiffType::Ascii,
        2,
        [b'N', 0, 0, 0],
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_GPS_DEST_DISTANCE,
        FrameExifTiffType::Rational,
        1,
        184u32.to_le_bytes(),
    );
    data.extend_from_slice(&0u32.to_le_bytes());

    for value in [33u32, 52, 7] {
        data.extend_from_slice(&value.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
    }
    for value in [151u32, 12, 9] {
        data.extend_from_slice(&value.to_le_bytes());
        data.extend_from_slice(&1u32.to_le_bytes());
    }
    data.extend_from_slice(&91u32.to_le_bytes());
    data.extend_from_slice(&2u32.to_le_bytes());
    data.extend_from_slice(&42u32.to_le_bytes());
    data.extend_from_slice(&1u32.to_le_bytes());
    data
}

fn exif_exposure_tags_fixture() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
    data.extend_from_slice(&8u32.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    push_exif_entry(
        &mut data,
        FrameExifIfdPointerKind::EXIF_TAG,
        FrameExifTiffType::Long,
        1,
        26u32.to_le_bytes(),
    );
    data.extend_from_slice(&0u32.to_le_bytes());

    data.extend_from_slice(&7u16.to_le_bytes());
    push_exif_entry(
        &mut data,
        FrameExif::TAG_EXPOSURE_TIME,
        FrameExifTiffType::Rational,
        1,
        116u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_F_NUMBER,
        FrameExifTiffType::Rational,
        1,
        124u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_EXPOSURE_BIAS_VALUE,
        FrameExifTiffType::SignedRational,
        1,
        132u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_FOCAL_LENGTH,
        FrameExifTiffType::Rational,
        1,
        140u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_PIXEL_X_DIMENSION,
        FrameExifTiffType::Long,
        1,
        1920u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_PIXEL_Y_DIMENSION,
        FrameExifTiffType::Short,
        1,
        [0x38, 0x04, 0, 0],
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_DATE_TIME_DIGITIZED,
        FrameExifTiffType::Ascii,
        20,
        148u32.to_le_bytes(),
    );
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&1u32.to_le_bytes());
    data.extend_from_slice(&125u32.to_le_bytes());
    data.extend_from_slice(&28u32.to_le_bytes());
    data.extend_from_slice(&10u32.to_le_bytes());
    data.extend_from_slice(&(-1i32).to_le_bytes());
    data.extend_from_slice(&3i32.to_le_bytes());
    data.extend_from_slice(&50u32.to_le_bytes());
    data.extend_from_slice(&1u32.to_le_bytes());
    data.extend_from_slice(b"2026:05:04 12:35:00\0");
    data
}

fn exif_capture_settings_fixture() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
    data.extend_from_slice(&8u32.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    push_exif_entry(
        &mut data,
        FrameExifIfdPointerKind::EXIF_TAG,
        FrameExifTiffType::Long,
        1,
        26u32.to_le_bytes(),
    );
    data.extend_from_slice(&0u32.to_le_bytes());

    data.extend_from_slice(&7u16.to_le_bytes());
    push_exif_entry(
        &mut data,
        FrameExif::TAG_EXPOSURE_PROGRAM,
        FrameExifTiffType::Short,
        1,
        [3, 0, 0, 0],
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_METERING_MODE,
        FrameExifTiffType::Short,
        1,
        [5, 0, 0, 0],
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_LIGHT_SOURCE,
        FrameExifTiffType::Short,
        1,
        [21, 0, 0, 0],
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_FLASH,
        FrameExifTiffType::Short,
        1,
        [0x41, 0, 0, 0],
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_WHITE_BALANCE,
        FrameExifTiffType::Short,
        1,
        [1, 0, 0, 0],
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_DIGITAL_ZOOM_RATIO,
        FrameExifTiffType::Rational,
        1,
        116u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_FOCAL_LENGTH_IN_35MM_FILM,
        FrameExifTiffType::Short,
        1,
        [75, 0, 0, 0],
    );
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&3u32.to_le_bytes());
    data.extend_from_slice(&2u32.to_le_bytes());
    data
}

fn exif_descriptive_tags_fixture() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
    data.extend_from_slice(&8u32.to_le_bytes());
    data.extend_from_slice(&5u16.to_le_bytes());
    push_exif_entry(
        &mut data,
        FrameExif::TAG_IMAGE_DESCRIPTION,
        FrameExifTiffType::Ascii,
        13,
        74u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_SOFTWARE,
        FrameExifTiffType::Ascii,
        11,
        87u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_DATE_TIME,
        FrameExifTiffType::Ascii,
        20,
        98u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_ARTIST,
        FrameExifTiffType::Ascii,
        7,
        118u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExif::TAG_COPYRIGHT,
        FrameExifTiffType::Ascii,
        13,
        125u32.to_le_bytes(),
    );
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(b"Frame sample\0");
    data.extend_from_slice(b"ffmpegrust\0");
    data.extend_from_slice(b"2026:05:05 01:02:03\0");
    data.extend_from_slice(b"OpenAI\0");
    data.extend_from_slice(b"2026 Example\0");
    data
}

fn exif_with_linked_ifds_fixture() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&[0x49, 0x49, 0x2A, 0x00]);
    data.extend_from_slice(&8u32.to_le_bytes());
    data.extend_from_slice(&3u16.to_le_bytes());
    push_exif_entry(
        &mut data,
        0x010F,
        FrameExifTiffType::Ascii,
        6,
        50u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExifIfdPointerKind::EXIF_TAG,
        FrameExifTiffType::Long,
        1,
        56u32.to_le_bytes(),
    );
    push_exif_entry(
        &mut data,
        FrameExifIfdPointerKind::GPS_TAG,
        FrameExifTiffType::Long,
        1,
        74u32.to_le_bytes(),
    );
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(b"Rusty\0");
    data.extend_from_slice(&1u16.to_le_bytes());
    push_exif_entry(
        &mut data,
        FrameExifIfdPointerKind::INTEROPERABILITY_TAG,
        FrameExifTiffType::Long,
        1,
        92u32.to_le_bytes(),
    );
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    push_exif_entry(&mut data, 0, FrameExifTiffType::Byte, 4, [2, 3, 0, 0]);
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    push_exif_entry(&mut data, 1, FrameExifTiffType::Ascii, 4, *b"R98\0");
    data.extend_from_slice(&0u32.to_le_bytes());
    data
}

fn push_exif_entry(
    data: &mut Vec<u8>,
    tag: u16,
    tiff_type: FrameExifTiffType,
    count: u32,
    value_or_offset: [u8; 4],
) {
    data.extend_from_slice(&tag.to_le_bytes());
    data.extend_from_slice(&tiff_type.raw().to_le_bytes());
    data.extend_from_slice(&count.to_le_bytes());
    data.extend_from_slice(&value_or_offset);
}

fn exercise_exif_entry_typed_values(entry: FrameExifEntry<'_>) {
    match entry.byte_values() {
        Ok(Some(values)) => {
            assert_eq!(entry.tiff_type(), FrameExifTiffType::Byte);
            assert_eq!(values.len(), entry.count() as usize);
        }
        Ok(None) => assert_ne!(entry.tiff_type(), FrameExifTiffType::Byte),
        Err(err) => assert_eq!(err.kind(), AvErrorKind::InvalidData),
    }

    match entry.ascii_strings() {
        Ok(Some(strings)) => {
            assert_eq!(entry.tiff_type(), FrameExifTiffType::Ascii);
            for value in strings {
                assert!(value.is_ascii());
            }
        }
        Ok(None) => assert_ne!(entry.tiff_type(), FrameExifTiffType::Ascii),
        Err(err) => {
            assert_eq!(entry.tiff_type(), FrameExifTiffType::Ascii);
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
        }
    }

    match entry.short_values() {
        Ok(Some(values)) => {
            assert_eq!(entry.tiff_type(), FrameExifTiffType::Short);
            assert_eq!(values.len(), entry.count() as usize);
        }
        Ok(None) => assert_ne!(entry.tiff_type(), FrameExifTiffType::Short),
        Err(err) => assert_eq!(err.kind(), AvErrorKind::InvalidData),
    }

    match entry.long_values() {
        Ok(Some(values)) => {
            assert!(matches!(
                entry.tiff_type(),
                FrameExifTiffType::Long | FrameExifTiffType::Ifd
            ));
            assert_eq!(values.len(), entry.count() as usize);
        }
        Ok(None) => assert!(!matches!(
            entry.tiff_type(),
            FrameExifTiffType::Long | FrameExifTiffType::Ifd
        )),
        Err(err) => assert_eq!(err.kind(), AvErrorKind::InvalidData),
    }

    match entry.signed_short_values() {
        Ok(Some(values)) => {
            assert_eq!(entry.tiff_type(), FrameExifTiffType::SignedShort);
            assert_eq!(values.len(), entry.count() as usize);
        }
        Ok(None) => assert_ne!(entry.tiff_type(), FrameExifTiffType::SignedShort),
        Err(err) => assert_eq!(err.kind(), AvErrorKind::InvalidData),
    }

    match entry.signed_long_values() {
        Ok(Some(values)) => {
            assert_eq!(entry.tiff_type(), FrameExifTiffType::SignedLong);
            assert_eq!(values.len(), entry.count() as usize);
        }
        Ok(None) => assert_ne!(entry.tiff_type(), FrameExifTiffType::SignedLong),
        Err(err) => assert_eq!(err.kind(), AvErrorKind::InvalidData),
    }

    match entry.rational_values() {
        Ok(Some(values)) => {
            assert_eq!(entry.tiff_type(), FrameExifTiffType::Rational);
            assert_eq!(values.len(), entry.count() as usize);
            for value in values {
                assert_ne!(value.denominator(), 0);
            }
        }
        Ok(None) => assert_ne!(entry.tiff_type(), FrameExifTiffType::Rational),
        Err(err) => {
            assert_eq!(entry.tiff_type(), FrameExifTiffType::Rational);
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
        }
    }

    match entry.signed_rational_values() {
        Ok(Some(values)) => {
            assert_eq!(entry.tiff_type(), FrameExifTiffType::SignedRational);
            assert_eq!(values.len(), entry.count() as usize);
            for value in values {
                assert_ne!(value.denominator(), 0);
            }
        }
        Ok(None) => assert_ne!(entry.tiff_type(), FrameExifTiffType::SignedRational),
        Err(err) => {
            assert_eq!(entry.tiff_type(), FrameExifTiffType::SignedRational);
            assert_eq!(err.kind(), AvErrorKind::InvalidData);
        }
    }
}

fn exif_payload_invalid(data: &[u8]) -> bool {
    if data.len() < FrameExif::TIFF_HEADER_LEN {
        return true;
    }

    let endian = if data.starts_with(&[0x49, 0x49, 0x2A, 0x00]) {
        FrameExifEndian::Little
    } else if data.starts_with(&[0x4D, 0x4D, 0x00, 0x2A]) {
        FrameExifEndian::Big
    } else {
        return true;
    };
    let offset = read_exif_u32(data, endian, 4) as usize;
    let mut seen_offsets = Vec::new();
    let mut linked_offsets = Vec::new();

    if exif_ifd_chain_invalid(data, endian, offset, &mut seen_offsets, &mut linked_offsets) {
        return true;
    }

    let mut linked_index = 0;
    let mut linked_count = 0;
    while linked_index < linked_offsets.len() {
        let offset = linked_offsets[linked_index];
        linked_index += 1;
        let seen_before = seen_offsets.len();
        if exif_ifd_chain_invalid(data, endian, offset, &mut seen_offsets, &mut linked_offsets) {
            return true;
        }
        linked_count += seen_offsets.len().saturating_sub(seen_before);
        if linked_count > FrameExif::MAX_LINKED_IFDS {
            return true;
        }
    }

    false
}

fn exif_ifd_chain_invalid(
    data: &[u8],
    endian: FrameExifEndian,
    mut offset: usize,
    seen_offsets: &mut Vec<usize>,
    linked_offsets: &mut Vec<usize>,
) -> bool {
    for _ in 0..FrameExif::MAX_IFDS {
        let Some(count_end) = offset.checked_add(FrameExif::IFD_COUNT_LEN) else {
            return true;
        };
        if offset < FrameExif::TIFF_HEADER_LEN
            || count_end > data.len()
            || seen_offsets.contains(&offset)
        {
            return true;
        }
        seen_offsets.push(offset);

        let entry_count = read_exif_u16(data, endian, offset) as usize;
        if entry_count > FrameExif::MAX_IFD_ENTRIES {
            return true;
        }
        let Some(entries_len) = entry_count.checked_mul(FrameExif::IFD_ENTRY_LEN) else {
            return true;
        };
        let Some(next_offset_position) = count_end.checked_add(entries_len) else {
            return true;
        };
        let Some(table_end) = next_offset_position.checked_add(FrameExif::NEXT_IFD_OFFSET_LEN)
        else {
            return true;
        };
        if table_end > data.len() {
            return true;
        }

        for index in 0..entry_count {
            let entry_offset = count_end + index * FrameExif::IFD_ENTRY_LEN;
            let tag = read_exif_u16(data, endian, entry_offset);
            let Ok(tiff_type) =
                FrameExifTiffType::from_raw(read_exif_u16(data, endian, entry_offset + 2))
            else {
                return true;
            };
            let count = read_exif_u32(data, endian, entry_offset + 4);
            let value_offset = read_exif_u32(data, endian, entry_offset + 8) as usize;
            let Some(data_len) = tiff_type.element_size().checked_mul(count as usize) else {
                return true;
            };
            if data_len > 4 {
                let Some(end) = value_offset.checked_add(data_len) else {
                    return true;
                };
                if end > data.len() {
                    return true;
                }
            }
            if FrameExifIfdPointerKind::from_tag(tag).is_some() {
                if !matches!(tiff_type, FrameExifTiffType::Long | FrameExifTiffType::Ifd)
                    || count != 1
                    || value_offset < FrameExif::TIFF_HEADER_LEN
                {
                    return true;
                }
                linked_offsets.push(value_offset);
            }
        }

        let next = read_exif_u32(data, endian, next_offset_position);
        if next == 0 {
            return false;
        }
        offset = next as usize;
    }

    true
}

fn read_exif_u16(data: &[u8], endian: FrameExifEndian, offset: usize) -> u16 {
    let mut raw = [0; 2];
    raw.copy_from_slice(&data[offset..offset + 2]);
    match endian {
        FrameExifEndian::Little => u16::from_le_bytes(raw),
        FrameExifEndian::Big => u16::from_be_bytes(raw),
    }
}

fn read_exif_u32(data: &[u8], endian: FrameExifEndian, offset: usize) -> u32 {
    let mut raw = [0; 4];
    raw.copy_from_slice(&data[offset..offset + 4]);
    match endian {
        FrameExifEndian::Little => u32::from_le_bytes(raw),
        FrameExifEndian::Big => u32::from_be_bytes(raw),
    }
}

fn minimal_film_grain_av1_fixture() -> Vec<u8> {
    let mut data = vec![0; FrameFilmGrainParams::DATA_LEN];
    write_ne_i32(
        &mut data,
        film_grain_type_field_offset(),
        FrameFilmGrainParamsType::Av1.as_raw(),
    );
    write_ne_u64(
        &mut data,
        film_grain_seed_field_offset(),
        0xAABB_CCDD_EEFF_0011,
    );
    write_ne_i32(&mut data, film_grain_width_field_offset(), 64);
    write_ne_i32(&mut data, film_grain_height_field_offset(), 64);
    write_ne_i32(&mut data, film_grain_bit_depth_luma_field_offset(), 8);
    write_ne_i32(&mut data, film_grain_bit_depth_chroma_field_offset(), 8);

    write_ne_i32(&mut data, film_grain_aom_num_y_points_offset(), 1);
    data[film_grain_aom_y_points_offset()] = 24;
    data[film_grain_aom_y_points_offset() + 1] = 9;
    write_ne_i32(&mut data, film_grain_aom_scaling_shift_offset(), 8);
    write_ne_i32(&mut data, film_grain_aom_ar_coeff_shift_offset(), 6);
    data
}

fn film_grain_params_payload_invalid(data: &[u8]) -> bool {
    if data.len() != FrameFilmGrainParams::DATA_LEN {
        return true;
    }

    if !matches!(read_ne_i32(data, film_grain_type_field_offset()), 0..=2) {
        return true;
    }
    for offset in [
        film_grain_width_field_offset(),
        film_grain_height_field_offset(),
        film_grain_subsampling_x_field_offset(),
        film_grain_subsampling_y_field_offset(),
        film_grain_bit_depth_luma_field_offset(),
        film_grain_bit_depth_chroma_field_offset(),
    ] {
        if read_ne_i32(data, offset) < 0 {
            return true;
        }
    }

    match read_ne_i32(data, film_grain_type_field_offset()) {
        0 => false,
        1 => film_grain_aom_payload_invalid(
            &data[film_grain_codec_field_offset()
                ..film_grain_codec_field_offset() + FrameFilmGrainAomParams::DATA_LEN],
        ),
        2 => film_grain_h274_payload_invalid(
            &data[film_grain_codec_field_offset()
                ..film_grain_codec_field_offset() + FrameFilmGrainH274Params::DATA_LEN],
        ),
        _ => true,
    }
}

fn film_grain_aom_payload_invalid(data: &[u8]) -> bool {
    if data.len() != FrameFilmGrainAomParams::DATA_LEN {
        return true;
    }

    let num_y_points = read_ne_i32(data, 0);
    if !(0..=FrameFilmGrainAomParams::Y_POINTS as i32).contains(&num_y_points) {
        return true;
    }
    if !matches!(read_ne_i32(data, 32), 0 | 1) {
        return true;
    }
    for plane in 0..FrameFilmGrainAomParams::UV_PLANES {
        if !(0..=FrameFilmGrainAomParams::UV_POINTS as i32)
            .contains(&read_ne_i32(data, 36 + plane * 4))
        {
            return true;
        }
    }
    if !(8..=11).contains(&read_ne_i32(data, 84)) {
        return true;
    }
    if !(0..=3).contains(&read_ne_i32(data, 88)) {
        return true;
    }
    if !(6..=9).contains(&read_ne_i32(data, 168)) {
        return true;
    }
    if !(0..=3).contains(&read_ne_i32(data, 172)) {
        return true;
    }
    for plane in 0..FrameFilmGrainAomParams::UV_PLANES {
        if !(-256..=255).contains(&read_ne_i32(data, 192 + plane * 4)) {
            return true;
        }
    }
    !matches!(read_ne_i32(data, 200), 0 | 1) || !matches!(read_ne_i32(data, 204), 0 | 1)
}

fn film_grain_h274_payload_invalid(data: &[u8]) -> bool {
    if data.len() != FrameFilmGrainH274Params::DATA_LEN {
        return true;
    }

    if !matches!(read_ne_i32(data, 0), 0 | 1) || !matches!(read_ne_i32(data, 4), 0 | 1) {
        return true;
    }

    for component in 0..FrameFilmGrainH274Params::COMPONENTS {
        let present = read_ne_i32(data, 12 + component * 4);
        if !matches!(present, 0 | 1) {
            return true;
        }
        let intervals = usize::from(read_ne_u16(data, 24 + component * 2));
        let values = usize::from(data[30 + component]);
        if present == 0 {
            if intervals != 0 || values != 0 {
                return true;
            }
            continue;
        }
        if !(1..=FrameFilmGrainH274Params::MAX_INTENSITY_INTERVALS).contains(&intervals) {
            return true;
        }
        if !(1..=FrameFilmGrainH274Params::MAX_MODEL_VALUES).contains(&values) {
            return true;
        }
        for interval in 0..intervals {
            let lower =
                data[33 + component * FrameFilmGrainH274Params::MAX_INTENSITY_INTERVALS + interval];
            let upper = data
                [801 + component * FrameFilmGrainH274Params::MAX_INTENSITY_INTERVALS + interval];
            if lower > upper {
                return true;
            }
        }
    }

    false
}

fn film_grain_type_field_offset() -> usize {
    0
}

fn film_grain_seed_field_offset() -> usize {
    align_up_usize(
        film_grain_type_field_offset() + 4,
        core::mem::align_of::<u64>(),
    )
}

fn film_grain_width_field_offset() -> usize {
    film_grain_seed_field_offset() + 8
}

fn film_grain_height_field_offset() -> usize {
    film_grain_width_field_offset() + 4
}

fn film_grain_subsampling_x_field_offset() -> usize {
    film_grain_height_field_offset() + 4
}

fn film_grain_subsampling_y_field_offset() -> usize {
    film_grain_subsampling_x_field_offset() + 4
}

fn film_grain_bit_depth_luma_field_offset() -> usize {
    film_grain_subsampling_y_field_offset() + 24
}

fn film_grain_bit_depth_chroma_field_offset() -> usize {
    film_grain_bit_depth_luma_field_offset() + 4
}

fn film_grain_codec_field_offset() -> usize {
    film_grain_bit_depth_chroma_field_offset() + 4
}

fn film_grain_aom_num_y_points_offset() -> usize {
    film_grain_codec_field_offset()
}

fn film_grain_aom_y_points_offset() -> usize {
    film_grain_codec_field_offset() + 4
}

fn film_grain_aom_scaling_shift_offset() -> usize {
    film_grain_codec_field_offset() + 84
}

fn film_grain_aom_ar_coeff_shift_offset() -> usize {
    film_grain_codec_field_offset() + 168
}

fn minimal_detection_bboxes_fixture() -> Vec<u8> {
    let mut data = vec![0; FrameDetectionBboxes::HEADER_LEN + FrameDetectionBbox::DATA_LEN];
    write_fixed_bytes(
        &mut data,
        0,
        FrameDetectionBboxes::SOURCE_LEN,
        b"fuzz-detector",
    );
    write_ne_u32(&mut data, detection_bboxes_nb_bboxes_field_offset(), 1);
    write_ne_usize(
        &mut data,
        detection_bboxes_bboxes_offset_field_offset(),
        FrameDetectionBboxes::BBOXES_OFFSET,
    );
    write_ne_usize(
        &mut data,
        detection_bboxes_bbox_size_field_offset(),
        FrameDetectionBbox::DATA_LEN,
    );

    let bbox = FrameDetectionBboxes::BBOXES_OFFSET;
    write_ne_i32(&mut data, bbox + detection_bbox_x_field_offset(), 1);
    write_ne_i32(&mut data, bbox + detection_bbox_y_field_offset(), 2);
    write_ne_i32(&mut data, bbox + detection_bbox_width_field_offset(), 3);
    write_ne_i32(&mut data, bbox + detection_bbox_height_field_offset(), 4);
    write_fixed_bytes(
        &mut data,
        bbox + detection_bbox_detect_label_field_offset(),
        FrameDetectionBbox::LABEL_LEN,
        b"object",
    );
    write_ne_rational(
        &mut data,
        bbox + detection_bbox_detect_confidence_field_offset(),
        Rational::from_raw(1, 2),
    );
    write_ne_u32(
        &mut data,
        bbox + detection_bbox_classify_count_field_offset(),
        1,
    );
    write_fixed_bytes(
        &mut data,
        bbox + detection_bbox_classify_labels_field_offset(),
        FrameDetectionBbox::LABEL_LEN,
        b"class",
    );
    write_ne_rational(
        &mut data,
        bbox + detection_bbox_classify_confidences_field_offset(),
        Rational::from_raw(3, 4),
    );

    data
}

fn detection_bboxes_payload_invalid(data: &[u8]) -> bool {
    if data.len() < FrameDetectionBboxes::HEADER_LEN {
        return true;
    }
    if read_ne_usize(data, detection_bboxes_bboxes_offset_field_offset())
        != FrameDetectionBboxes::BBOXES_OFFSET
    {
        return true;
    }
    if read_ne_usize(data, detection_bboxes_bbox_size_field_offset())
        != FrameDetectionBbox::DATA_LEN
    {
        return true;
    }

    let nb_bboxes = read_ne_u32(data, detection_bboxes_nb_bboxes_field_offset()) as usize;
    let Some(bboxes_len) = nb_bboxes.checked_mul(FrameDetectionBbox::DATA_LEN) else {
        return true;
    };
    let Some(expected_len) = FrameDetectionBboxes::BBOXES_OFFSET.checked_add(bboxes_len) else {
        return true;
    };
    if data.len() != expected_len {
        return true;
    }

    for bbox in
        data[FrameDetectionBboxes::BBOXES_OFFSET..].chunks_exact(FrameDetectionBbox::DATA_LEN)
    {
        if read_ne_u32(bbox, detection_bbox_classify_count_field_offset()) as usize
            > FrameDetectionBbox::MAX_CLASSIFICATIONS
        {
            return true;
        }
    }

    false
}

fn minimal_dolby_vision_metadata_fixture() -> Vec<u8> {
    let mut data = vec![0; FrameDolbyVisionMetadata::DATA_LEN];
    write_ne_usize(
        &mut data,
        dovi_header_offset_field_offset(),
        FrameDolbyVisionMetadata::HEADER_OFFSET,
    );
    write_ne_usize(
        &mut data,
        dovi_mapping_offset_field_offset(),
        FrameDolbyVisionMetadata::MAPPING_OFFSET,
    );
    write_ne_usize(
        &mut data,
        dovi_color_offset_field_offset(),
        FrameDolbyVisionMetadata::COLOR_OFFSET,
    );
    write_ne_usize(
        &mut data,
        dovi_ext_block_offset_field_offset(),
        FrameDolbyVisionMetadata::EXT_BLOCK_OFFSET,
    );
    write_ne_usize(
        &mut data,
        dovi_ext_block_size_field_offset(),
        FrameDolbyVisionMetadata::EXT_BLOCK_SIZE,
    );
    write_ne_i32(&mut data, dovi_num_ext_blocks_field_offset(), 2);

    let header = FrameDolbyVisionMetadata::HEADER_OFFSET;
    data[header] = 2;
    write_ne_u16(&mut data, header + 2, 18);
    data[header + 4] = 8;
    data[header + 5] = 6;
    data[header + 7] = 1;
    data[header + 8] = 28;
    data[header + 11] = 10;
    data[header + 12] = 10;
    data[header + 13] = 12;
    data[header + 16] = 1;
    data[header + 17] = 4;

    let mapping = FrameDolbyVisionMetadata::MAPPING_OFFSET;
    data[mapping] = 3;
    data[mapping + 1] = 1;
    data[mapping + 2] = 2;
    write_ne_i32(&mut data, mapping + dovi_mapping_nlq_method_offset(), 0);
    write_ne_u32(
        &mut data,
        mapping + dovi_mapping_num_x_partitions_offset(),
        1,
    );
    write_ne_u32(
        &mut data,
        mapping + dovi_mapping_num_y_partitions_offset(),
        1,
    );

    let color = FrameDolbyVisionMetadata::COLOR_OFFSET;
    data[color] = 9;
    data[color + 1] = 1;
    write_ne_rational(&mut data, color + 4, Rational::from_raw(1, 2));
    write_ne_u16(&mut data, color + dovi_color_signal_eotf_offset(), 2084);
    data[color + dovi_color_signal_full_range_offset()] = 3;

    let level1 = FrameDolbyVisionMetadata::EXT_BLOCK_OFFSET;
    data[level1] = 1;
    write_ne_u16(&mut data, level1 + 4, 10);
    write_ne_u16(&mut data, level1 + 6, 2048);
    write_ne_u16(&mut data, level1 + 8, 512);

    let level6 = level1 + FrameDolbyVisionMetadata::EXT_BLOCK_SIZE;
    data[level6] = 6;
    write_ne_u16(&mut data, level6 + 4, 1000);
    write_ne_u16(&mut data, level6 + 6, 1);
    write_ne_u16(&mut data, level6 + 8, 800);
    write_ne_u16(&mut data, level6 + 10, 400);

    data
}

fn dolby_vision_metadata_payload_invalid(data: &[u8]) -> bool {
    if data.len() != FrameDolbyVisionMetadata::DATA_LEN {
        return true;
    }
    if read_ne_usize(data, dovi_header_offset_field_offset())
        != FrameDolbyVisionMetadata::HEADER_OFFSET
    {
        return true;
    }
    if read_ne_usize(data, dovi_mapping_offset_field_offset())
        != FrameDolbyVisionMetadata::MAPPING_OFFSET
    {
        return true;
    }
    if read_ne_usize(data, dovi_color_offset_field_offset())
        != FrameDolbyVisionMetadata::COLOR_OFFSET
    {
        return true;
    }
    if read_ne_usize(data, dovi_ext_block_offset_field_offset())
        != FrameDolbyVisionMetadata::EXT_BLOCK_OFFSET
    {
        return true;
    }
    if read_ne_usize(data, dovi_ext_block_size_field_offset())
        != FrameDolbyVisionMetadata::EXT_BLOCK_SIZE
    {
        return true;
    }
    let count = read_ne_i32(data, dovi_num_ext_blocks_field_offset());
    if !(0..=FrameDolbyVisionMetadata::MAX_EXT_BLOCKS as i32).contains(&count) {
        return true;
    }
    for index in 0..count as usize {
        let offset = FrameDolbyVisionMetadata::EXT_BLOCK_OFFSET
            + index * FrameDolbyVisionMetadata::EXT_BLOCK_SIZE;
        if data[offset] == 0 {
            return true;
        }
    }
    if data[FrameDolbyVisionMetadata::COLOR_OFFSET + dovi_color_signal_full_range_offset()] > 3 {
        return true;
    }

    false
}

fn dovi_header_offset_field_offset() -> usize {
    0
}

fn dovi_mapping_offset_field_offset() -> usize {
    core::mem::size_of::<usize>()
}

fn dovi_color_offset_field_offset() -> usize {
    2 * core::mem::size_of::<usize>()
}

fn dovi_ext_block_offset_field_offset() -> usize {
    3 * core::mem::size_of::<usize>()
}

fn dovi_ext_block_size_field_offset() -> usize {
    4 * core::mem::size_of::<usize>()
}

fn dovi_num_ext_blocks_field_offset() -> usize {
    5 * core::mem::size_of::<usize>()
}

fn dovi_mapping_nlq_method_offset() -> usize {
    5_024
}

fn dovi_mapping_num_x_partitions_offset() -> usize {
    dovi_mapping_nlq_method_offset() + 4
}

fn dovi_mapping_num_y_partitions_offset() -> usize {
    dovi_mapping_num_x_partitions_offset() + 4
}

fn dovi_color_signal_eotf_offset() -> usize {
    172
}

fn dovi_color_signal_full_range_offset() -> usize {
    187
}

fn detection_bboxes_nb_bboxes_field_offset() -> usize {
    FrameDetectionBboxes::SOURCE_LEN
}

fn detection_bboxes_bboxes_offset_field_offset() -> usize {
    align_up_usize(
        detection_bboxes_nb_bboxes_field_offset() + 4,
        core::mem::align_of::<usize>(),
    )
}

fn detection_bboxes_bbox_size_field_offset() -> usize {
    detection_bboxes_bboxes_offset_field_offset() + core::mem::size_of::<usize>()
}

fn detection_bboxes_first_classify_count_offset() -> usize {
    FrameDetectionBboxes::BBOXES_OFFSET + detection_bbox_classify_count_field_offset()
}

fn detection_bbox_x_field_offset() -> usize {
    0
}

fn detection_bbox_y_field_offset() -> usize {
    4
}

fn detection_bbox_width_field_offset() -> usize {
    8
}

fn detection_bbox_height_field_offset() -> usize {
    12
}

fn detection_bbox_detect_label_field_offset() -> usize {
    16
}

fn detection_bbox_detect_confidence_field_offset() -> usize {
    detection_bbox_detect_label_field_offset() + FrameDetectionBbox::LABEL_LEN
}

fn detection_bbox_classify_count_field_offset() -> usize {
    detection_bbox_detect_confidence_field_offset() + 8
}

fn detection_bbox_classify_labels_field_offset() -> usize {
    detection_bbox_classify_count_field_offset() + 4
}

fn detection_bbox_classify_confidences_field_offset() -> usize {
    detection_bbox_classify_labels_field_offset()
        + FrameDetectionBbox::MAX_CLASSIFICATIONS * FrameDetectionBbox::LABEL_LEN
}

fn align_up_usize(value: usize, align: usize) -> usize {
    let remainder = value % align;
    if remainder == 0 {
        value
    } else {
        value + align - remainder
    }
}

fn minimal_icc_profile_fixture() -> Vec<u8> {
    let mut data = vec![0; FrameIccProfile::MIN_DATA_LEN];
    data[0..4].copy_from_slice(&(FrameIccProfile::MIN_DATA_LEN as u32).to_be_bytes());
    data[8..12].copy_from_slice(&0x0430_0000u32.to_be_bytes());
    data[12..16].copy_from_slice(b"mntr");
    data[16..20].copy_from_slice(b"RGB ");
    data[20..24].copy_from_slice(b"XYZ ");
    data[36..40].copy_from_slice(&FrameIccProfile::ICC_SIGNATURE);
    data[128..132].copy_from_slice(&0u32.to_be_bytes());
    data
}

fn icc_profile_payload_invalid(data: &[u8]) -> bool {
    if data.len() < FrameIccProfile::MIN_DATA_LEN {
        return true;
    }

    let declared_size = read_be_u32(data, 0);
    if usize::try_from(declared_size).ok() != Some(data.len()) {
        return true;
    }

    if data[36..40] != FrameIccProfile::ICC_SIGNATURE {
        return true;
    }

    let tag_count = read_be_u32(data, FrameIccProfile::TAG_COUNT_OFFSET);
    let Some(tag_table_len) = usize::try_from(tag_count)
        .ok()
        .and_then(|count| count.checked_mul(FrameIccProfile::TAG_RECORD_LEN))
        .and_then(|records_len| FrameIccProfile::MIN_DATA_LEN.checked_add(records_len))
    else {
        return true;
    };

    tag_table_len > data.len()
}

fn read_be_u32(data: &[u8], offset: usize) -> u32 {
    let mut raw = [0; 4];
    raw.copy_from_slice(&data[offset..offset + 4]);
    u32::from_be_bytes(raw)
}

fn read_ne_u32(data: &[u8], offset: usize) -> u32 {
    let mut raw = [0; 4];
    raw.copy_from_slice(&data[offset..offset + 4]);
    u32::from_ne_bytes(raw)
}

fn read_ne_u16(data: &[u8], offset: usize) -> u16 {
    let mut raw = [0; 2];
    raw.copy_from_slice(&data[offset..offset + 2]);
    u16::from_ne_bytes(raw)
}

fn read_ne_usize(data: &[u8], offset: usize) -> usize {
    let mut raw = [0; core::mem::size_of::<usize>()];
    raw.copy_from_slice(&data[offset..offset + core::mem::size_of::<usize>()]);
    usize::from_ne_bytes(raw)
}

fn read_ne_i32(data: &[u8], offset: usize) -> i32 {
    let mut raw = [0; 4];
    raw.copy_from_slice(&data[offset..offset + 4]);
    i32::from_ne_bytes(raw)
}

fn read_ne_rational(data: &[u8], offset: usize) -> Rational {
    Rational::from_raw(read_ne_i32(data, offset), read_ne_i32(data, offset + 4))
}

fn write_ne_u32(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
}

fn write_ne_u16(data: &mut [u8], offset: usize, value: u16) {
    data[offset..offset + 2].copy_from_slice(&value.to_ne_bytes());
}

fn write_ne_u64(data: &mut [u8], offset: usize, value: u64) {
    data[offset..offset + 8].copy_from_slice(&value.to_ne_bytes());
}

fn write_ne_usize(data: &mut [u8], offset: usize, value: usize) {
    data[offset..offset + core::mem::size_of::<usize>()].copy_from_slice(&value.to_ne_bytes());
}

fn write_ne_i32(data: &mut [u8], offset: usize, value: i32) {
    data[offset..offset + 4].copy_from_slice(&value.to_ne_bytes());
}

fn write_ne_rational(data: &mut [u8], offset: usize, value: Rational) {
    write_ne_i32(data, offset, value.num());
    write_ne_i32(data, offset + 4, value.den());
}

fn write_fixed_bytes(data: &mut [u8], offset: usize, len: usize, value: &[u8]) {
    assert!(value.len() < len);
    data[offset..offset + value.len()].copy_from_slice(value);
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

fn read_rational_from_payload(data: &[u8], offset: &mut usize) -> Rational {
    let mut num = [0; 4];
    num.copy_from_slice(&data[*offset..*offset + 4]);
    *offset += 4;
    let mut den = [0; 4];
    den.copy_from_slice(&data[*offset..*offset + 4]);
    *offset += 4;
    Rational::from_raw(i32::from_ne_bytes(num), i32::from_ne_bytes(den))
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
