#![no_main]

use avutil::{
    adler32, crc32_ieee, digest_to_hex, md5, rescale_q, rescale_q_rnd, rescale_q_rnd_pass_minmax,
    sha224, sha256, sha384, sha512, Adler32, AudioFrame, AvError, AvErrorKind, BufferPool,
    BufferPoolCallbacks, BufferRef, Channel, ChannelLayout, Crc32, Frame, FrameData, Md5, Packet,
    PacketFlags, PixelFormat, Rational, Rounding, SampleFormat, Sha224, Sha256, Sha384, Sha512,
    SideData, VideoFrame,
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
    assert!(!buffer.is_writable());
    assert!(buffer.get_mut().is_none());
    if !buffer.is_empty() {
        let original = buffer.as_slice()[0];
        buffer.make_mut()[0] = original.wrapping_add(1);
        assert_eq!(buffer.as_slice()[0], original.wrapping_add(1));
        assert_eq!(shared.as_slice(), payload.as_slice());
    } else {
        assert_eq!(buffer.make_mut(), &mut []);
        assert_eq!(shared.as_slice(), payload.as_slice());
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
    assert_eq!(&padded.as_padded_slice()[..payload.len()], payload.as_slice());
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

    let wrong_shape = BufferRef::zeroed_with_padding(payload_len.saturating_add(1), padding_len)
        .unwrap();
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
    assert!(auto_reused
        .as_padded_slice()
        .iter()
        .all(|byte| *byte == 0));
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
    assert_eq!(*released.lock().unwrap(), vec![release_storage]);

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

    let mut frame = Frame::video(video);
    let pts = timestamp_from(cursor.next());
    frame.set_pts(pts);
    assert_eq!(frame.pts(), pts);
    assert!(matches!(frame.data(), FrameData::Video(_)));

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
    let sample_format = SampleFormat::S16;

    assert_eq!(
        SampleFormat::from_name(sample_format.name()),
        Some(sample_format)
    );
    assert!(!sample_format.is_planar());

    let Ok(plane_sizes) = sample_format.plane_sizes(samples_per_channel, channels) else {
        assert_eq!(channels, 0);
        assert!(AudioFrame::new(sample_rate, channels, sample_format, 1, vec![vec![0]]).is_err());
        return;
    };
    assert_eq!(
        sample_format.plane_count(channels).unwrap(),
        plane_sizes.len()
    );
    assert_eq!(sample_format.bytes_per_sample(), 2);
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
    assert_eq!(
        audio.channel_layout(),
        ChannelLayout::default_for_count(channels)
    );

    let mut frame = Frame::audio(audio);
    frame.set_pts(timestamp_from(cursor.next()));
    assert!(matches!(frame.data(), FrameData::Audio(_)));

    let layout = channel_layout_from(cursor.next());
    if layout.channel_count() == channels {
        let frame = AudioFrame::new_with_channel_layout(
            sample_rate,
            layout,
            sample_format,
            samples_per_channel,
            planes,
        )
        .unwrap();
        assert_eq!(frame.channel_layout(), Some(layout));
        assert_eq!(frame.line_sizes(), plane_sizes.as_slice());
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
    assert_eq!(SampleFormat::S16.plane_sizes(2, 2).unwrap(), vec![8]);
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

fn expected_video_line_sizes(pixel_format: PixelFormat, width: usize) -> Vec<usize> {
    match pixel_format {
        PixelFormat::Gray8 => vec![width],
        PixelFormat::Rgb24 => vec![width * 3],
        PixelFormat::Rgba => vec![width * 4],
        PixelFormat::Yuv420p => vec![width, width / 2, width / 2],
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
