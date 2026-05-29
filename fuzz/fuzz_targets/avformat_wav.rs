#![no_main]

use avformat::WavDemuxer;
use avutil::SampleFormat;
use libfuzzer_sys::fuzz_target;

const VALID_WAV: &[u8] = b"RIFF,\0\0\0WAVEfmt \x10\0\0\0\x01\0\x02\0\x80\xbb\0\0\0\xee\x02\0\x04\0\x10\0data\x08\0\0\0\0\0\x01\0\x02\0\x03\0";
const TOO_SMALL_RIFF_WAV: &[u8] = b"RIFF\0\0\0\0";

fuzz_target!(|data: &[u8]| {
    exercise_wav(data);
    exercise_wav(VALID_WAV);
    exercise_wav(TOO_SMALL_RIFF_WAV);
    exercise_wav(&wave_format_extensible_pcm_wav());
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
