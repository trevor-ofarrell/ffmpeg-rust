#![no_main]

use avutil::{BitReader, BitWriter};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut reader = BitReader::new(data);

    for &op in data {
        let before = reader.bit_position();
        let requested = op & 0x7f;
        let result = match op & 0x03 {
            0 => reader.read_bits(requested),
            1 => reader.peek_bits(requested),
            2 => reader.skip_bits(usize::from(requested)).map(|_| 0),
            _ => reader.byte_align().map(|_| 0),
        };

        assert!(reader.bit_position() <= reader.len_bits());
        if result.is_err() {
            assert_eq!(reader.bit_position(), before);
        }
        if op & 0x03 == 1 {
            assert_eq!(reader.bit_position(), before);
        }
    }

    assert_eq!(reader.is_eof(), reader.bits_remaining() == 0);

    let mut writer = BitWriter::new();
    for chunk in data.chunks(9) {
        let width = chunk.first().copied().unwrap_or(0) & 0x7f;
        let mut bytes = [0_u8; 8];
        for (index, byte) in chunk.iter().skip(1).take(8).enumerate() {
            bytes[index] = *byte;
        }
        let value = u64::from_le_bytes(bytes);
        let before = writer.bit_position();
        let result = writer.write_bits(value, width);

        assert!(writer.bit_position() >= before);
        if result.is_err() {
            assert_eq!(writer.bit_position(), before);
        }
        assert_eq!(writer.is_empty(), writer.bit_position() == 0);
    }
    writer.byte_align_zero();
    assert!(writer.is_aligned());
});
