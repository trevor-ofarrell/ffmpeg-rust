#![no_main]

use avformat::{WavDemuxer, WavMuxer};
use avutil::SampleFormat;
use libfuzzer_sys::fuzz_target;

const VALID_WAV: &[u8] = b"RIFF,\0\0\0WAVEfmt \x10\0\0\0\x01\0\x02\0\x80\xbb\0\0\0\xee\x02\0\x04\0\x10\0data\x08\0\0\0\0\0\x01\0\x02\0\x03\0";
const TOO_SMALL_RIFF_WAV: &[u8] = b"RIFF\0\0\0\0";
const SHORT_PCM_FMT_WAV: &[u8] = b"RIFF\x14\0\0\0WAVEfmt \x08\0\0\0\x01\0\x01\0\0\0\0\0";

fuzz_target!(|data: &[u8]| {
    exercise_wav(data);
    exercise_wav(VALID_WAV);
    exercise_wav(TOO_SMALL_RIFF_WAV);
    exercise_wav(&wave_format_extensible_pcm_wav());
    let duplicate_data = wav_with_duplicate_data_chunks();
    exercise_wav(&duplicate_data);
    let duplicate_fmt = wav_with_duplicate_fmt_chunks();
    exercise_wav(&duplicate_fmt);
    exercise_duplicate_fmt_wav(&duplicate_fmt);
    let short_duplicate_fmt = wav_with_short_second_duplicate_fmt_chunk();
    exercise_wav(&short_duplicate_fmt);
    exercise_short_second_duplicate_fmt_wav(&short_duplicate_fmt);
    let empty_generated_wav = empty_generated_wav();
    exercise_wav(&empty_generated_wav);
    assert!(WavDemuxer::open(SHORT_PCM_FMT_WAV).is_err());
    assert_rejects_wav(&wave_with_missing_padding_after_odd_unknown_chunk());
    assert_rejects_wav(&wav_with_missing_padding_after_odd_fmt_chunk());
});

fn exercise_wav(input: &[u8]) {
    let Ok(mut demuxer) = WavDemuxer::open(input) else {
        return;
    };

    let info = demuxer.info().clone();
    assert_eq!(info.sample_format(), SampleFormat::S16);
    assert_eq!(info.bits_per_sample(), 16);
    assert_eq!(
        usize::from(info.block_align()),
        usize::from(info.channels()) * 2
    );
    assert_eq!(
        info.byte_rate(),
        info.sample_rate() * u32::from(info.block_align())
    );
    assert_eq!(
        info.samples_per_channel(),
        info.data_size() / usize::from(info.block_align())
    );

    if info.data_size() == 0 {
        assert!(demuxer.read_packet().unwrap().is_none());
        return;
    }

    let mut seen = 0usize;
    while let Ok(Some(packet)) = demuxer.read_packet() {
        assert_eq!(packet.stream_index(), 0);
        assert_eq!(packet.pts(), Some(0));
        assert_eq!(packet.dts(), Some(0));
        assert_eq!(
            packet.duration() as usize,
            packet.data().len() / usize::from(info.block_align())
        );
        seen = seen.saturating_add(packet.data().len());
    }
    assert_eq!(seen, info.data_size());
}

fn assert_rejects_wav(input: &[u8]) {
    assert!(WavDemuxer::open(input).is_err());
}

fn exercise_duplicate_fmt_wav(input: &[u8]) {
    let mut demuxer = WavDemuxer::open(input).expect("duplicate fmt WAV should open");
    let info = demuxer.info().clone();

    assert_eq!(info.channels(), 1);
    assert_eq!(info.sample_rate(), 44_100);
    assert_eq!(info.byte_rate(), 88_200);
    assert_eq!(info.block_align(), 2);
    assert_eq!(info.samples_per_channel(), 4);

    let packet = demuxer
        .read_packet()
        .expect("duplicate fmt WAV should yield a packet")
        .expect("duplicate fmt WAV should produce one packet");
    assert_eq!(packet.data().len(), 8);
    assert_eq!(packet.duration(), 4);
    assert!(demuxer.read_packet().unwrap().is_none());
}

fn exercise_short_second_duplicate_fmt_wav(input: &[u8]) {
    let mut demuxer =
        WavDemuxer::open(input).expect("duplicate fmt WAV with short second fmt should open");
    let info = demuxer.info().clone();

    assert_eq!(info.channels(), 1);
    assert_eq!(info.sample_rate(), 44_100);
    assert_eq!(info.byte_rate(), 88_200);
    assert_eq!(info.block_align(), 2);
    assert_eq!(info.samples_per_channel(), 2);

    let packet = demuxer
        .read_packet()
        .expect("duplicate fmt WAV with short second fmt should yield a packet")
        .expect("duplicate fmt WAV with short second fmt should produce one packet");
    assert_eq!(packet.data().len(), 4);
    assert_eq!(packet.duration(), 2);
    assert!(demuxer.read_packet().unwrap().is_none());
}

fn wave_with_missing_padding_after_odd_unknown_chunk() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&0_u32.to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16_u32.to_le_bytes());
    out.extend_from_slice(&[1, 0, 1, 0, 0x44, 0xAC, 0x00, 0x00]);
    out.extend_from_slice(&[0x88, 0x58, 0x01, 0x00, 2, 0, 16, 0]);
    out.extend_from_slice(b"JUNK");
    out.extend_from_slice(&3_u32.to_le_bytes());
    out.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
    out.extend_from_slice(b"data");
    out.extend_from_slice(&4_u32.to_le_bytes());
    out.extend_from_slice(&[0x00, 0x00, 0x01, 0x00]);
    let riff_size = out.len() - 8;
    out[4..8].copy_from_slice(&u32::try_from(riff_size).unwrap().to_le_bytes());
    out
}

fn wav_with_missing_padding_after_odd_fmt_chunk() -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&0_u32.to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&17_u32.to_le_bytes());
    out.extend_from_slice(&[1, 0, 1, 0, 0x44, 0xAC, 0x00, 0x00]);
    out.extend_from_slice(&[0x88, 0x58, 0x01, 0x00, 2, 0, 16, 0, 0]);
    out.extend_from_slice(b"data");
    out.extend_from_slice(&2_u32.to_le_bytes());
    out.extend_from_slice(&[1, 0]);
    let riff_size = out.len() - 8;
    out[4..8].copy_from_slice(&u32::try_from(riff_size).unwrap().to_le_bytes());
    out
}

fn empty_generated_wav() -> Vec<u8> {
    let mut muxer = WavMuxer::new_pcm_s16le(1, 44_100).expect("empty WAV parameters are valid");
    muxer.finish().expect("empty WAV should render")
}

fn wave_format_extensible_pcm_wav() -> Vec<u8> {
    let sample_rate: u32 = 44_100;
    let channels: u16 = 2;
    let block_align: u16 = 4;
    let byte_rate: u32 = sample_rate * u32::from(block_align);
    let data: &[u8] = &[0x00, 0x00, 0x01, 0x00];

    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(60 + u32::try_from(data.len()).unwrap()).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&40_u32.to_le_bytes());
    out.extend_from_slice(&0xFFFE_u16.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&sample_rate.to_le_bytes());
    out.extend_from_slice(&byte_rate.to_le_bytes());
    out.extend_from_slice(&block_align.to_le_bytes());
    out.extend_from_slice(&16_u16.to_le_bytes());
    out.extend_from_slice(&22_u16.to_le_bytes());
    out.extend_from_slice(&16_u16.to_le_bytes());
    out.extend_from_slice(&3_u32.to_le_bytes());
    out.extend_from_slice(&[
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B,
        0x71,
    ]);
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(u32::try_from(data.len()).unwrap()).to_le_bytes());
    out.extend_from_slice(data);

    out
}

fn wav_with_duplicate_data_chunks() -> Vec<u8> {
    let channels: u16 = 2;
    let sample_rate: u32 = 44_100;
    let block_align = channels * 2u16;
    let byte_rate = sample_rate * u32::from(block_align);
    let first_payload: &[u8] = b"\x00\x00\x01\x00\x02\x00\x03\x00";
    let second_payload: &[u8] = b"\xAA\x00\xBB\x00";

    let mut body = Vec::new();
    body.extend_from_slice(b"fmt ");
    body.extend_from_slice(&16_u32.to_le_bytes());
    body.extend_from_slice(&1_u16.to_le_bytes());
    body.extend_from_slice(&channels.to_le_bytes());
    body.extend_from_slice(&sample_rate.to_le_bytes());
    body.extend_from_slice(&byte_rate.to_le_bytes());
    body.extend_from_slice(&block_align.to_le_bytes());
    body.extend_from_slice(&16_u16.to_le_bytes());

    body.extend_from_slice(b"data");
    body.extend_from_slice(&(u32::try_from(first_payload.len()).unwrap()).to_le_bytes());
    body.extend_from_slice(first_payload);

    body.extend_from_slice(b"data");
    body.extend_from_slice(&(u32::try_from(second_payload.len()).unwrap()).to_le_bytes());
    body.extend_from_slice(second_payload);

    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(u32::try_from(body.len() + 4).unwrap()).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(&body);
    out
}

fn wav_with_duplicate_fmt_chunks() -> Vec<u8> {
    let first_channels: u16 = 1;
    let first_sample_rate: u32 = 44_100;
    let second_channels: u16 = 2;
    let second_sample_rate: u32 = 48_000;
    let block_align = first_channels * 2u16;
    let byte_rate = first_sample_rate * u32::from(block_align);
    let second_block_align = second_channels * 2u16;
    let second_byte_rate = second_sample_rate * u32::from(second_block_align);
    let data: &[u8] = &[0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00];

    let mut body = Vec::new();
    body.extend_from_slice(b"fmt ");
    body.extend_from_slice(&16_u32.to_le_bytes());
    body.extend_from_slice(&1_u16.to_le_bytes());
    body.extend_from_slice(&first_channels.to_le_bytes());
    body.extend_from_slice(&first_sample_rate.to_le_bytes());
    body.extend_from_slice(&byte_rate.to_le_bytes());
    body.extend_from_slice(&block_align.to_le_bytes());
    body.extend_from_slice(&16_u16.to_le_bytes());

    body.extend_from_slice(b"fmt ");
    body.extend_from_slice(&16_u32.to_le_bytes());
    body.extend_from_slice(&1_u16.to_le_bytes());
    body.extend_from_slice(&second_channels.to_le_bytes());
    body.extend_from_slice(&second_sample_rate.to_le_bytes());
    body.extend_from_slice(&second_byte_rate.to_le_bytes());
    body.extend_from_slice(&second_block_align.to_le_bytes());
    body.extend_from_slice(&16_u16.to_le_bytes());

    body.extend_from_slice(b"data");
    body.extend_from_slice(&(u32::try_from(data.len()).unwrap()).to_le_bytes());
    body.extend_from_slice(data);

    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(u32::try_from(body.len() + 4).unwrap()).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(&body);
    out
}

fn wav_with_short_second_duplicate_fmt_chunk() -> Vec<u8> {
    let first_channels: u16 = 1;
    let first_sample_rate: u32 = 44_100;
    let block_align = first_channels * 2u16;
    let byte_rate = first_sample_rate * u32::from(block_align);
    let data: &[u8] = &[0x00, 0x00, 0x01, 0x00];

    let mut body = Vec::new();
    body.extend_from_slice(b"fmt ");
    body.extend_from_slice(&16_u32.to_le_bytes());
    body.extend_from_slice(&1_u16.to_le_bytes());
    body.extend_from_slice(&first_channels.to_le_bytes());
    body.extend_from_slice(&first_sample_rate.to_le_bytes());
    body.extend_from_slice(&byte_rate.to_le_bytes());
    body.extend_from_slice(&block_align.to_le_bytes());
    body.extend_from_slice(&16_u16.to_le_bytes());

    body.extend_from_slice(b"fmt ");
    body.extend_from_slice(&8_u32.to_le_bytes());
    body.extend_from_slice(&[1, 0, 1, 0, 0, 0, 0, 0]);

    body.extend_from_slice(b"data");
    body.extend_from_slice(&(u32::try_from(data.len()).unwrap()).to_le_bytes());
    body.extend_from_slice(data);

    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(u32::try_from(body.len() + 4).unwrap()).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(&body);
    out
}
