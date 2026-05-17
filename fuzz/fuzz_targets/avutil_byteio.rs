#![no_main]

use avutil::{AvResult, ByteReader, ByteWriter};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut reader = ByteReader::new(data);
    let mut control = ByteReader::new(data);
    let mut writer = ByteWriter::new();

    while let Ok(op) = control.read_u8() {
        let before = reader.position();
        let result = run_read_operation(&mut reader, op);

        assert!(reader.position() <= reader.len());
        if result.is_err() {
            assert_eq!(reader.position(), before);
        } else {
            assert!(reader.position() >= before);
        }

        run_write_operation(&mut writer, data, op);
    }

    assert!(reader.remaining() <= reader.len());
    assert_eq!(reader.is_eof(), reader.remaining() == 0);
    assert_eq!(writer.is_empty(), writer.as_slice().is_empty());
});

fn run_read_operation(reader: &mut ByteReader<'_>, op: u8) -> AvResult<()> {
    match op % 20 {
        0 => reader.read_u8().map(|_| ()),
        1 => reader.read_i8().map(|_| ()),
        2 => reader.read_u16_le().map(|_| ()),
        3 => reader.read_u16_be().map(|_| ()),
        4 => reader.read_i16_le().map(|_| ()),
        5 => reader.read_i16_be().map(|_| ()),
        6 => reader.read_u24_le().map(|_| ()),
        7 => reader.read_u24_be().map(|_| ()),
        8 => reader.read_u32_le().map(|_| ()),
        9 => reader.read_u32_be().map(|_| ()),
        10 => reader.read_i32_le().map(|_| ()),
        11 => reader.read_i32_be().map(|_| ()),
        12 => reader.read_u48_le().map(|_| ()),
        13 => reader.read_u48_be().map(|_| ()),
        14 => reader.read_u64_le().map(|_| ()),
        15 => reader.read_u64_be().map(|_| ()),
        16 => reader.read_i64_le().map(|_| ()),
        17 => reader.read_i64_be().map(|_| ()),
        18 => reader.skip(usize::from(op >> 4)),
        _ => reader.read_exact(usize::from(op >> 4)).map(|_| ()),
    }
}

fn run_write_operation(writer: &mut ByteWriter, data: &[u8], op: u8) {
    writer.write_u8(op);
    writer.write_i8(op as i8);
    writer.write_u16_le(u16::from(op));
    writer.write_u16_be(u16::from(op));
    writer.write_i16_le(i16::from(op));
    writer.write_i16_be(i16::from(op));

    let mut bytes = [0_u8; 4];
    for (index, byte) in data.iter().take(4).enumerate() {
        bytes[index] = *byte;
    }
    let value = u32::from_le_bytes(bytes);
    let _ = writer.write_u24_le(value);
    let _ = writer.write_u24_be(value);
    writer.write_u32_le(value);
    writer.write_u32_be(value);
    writer.write_i32_le(value as i32);
    writer.write_i32_be(value as i32);

    let mut wide_bytes = [0_u8; 8];
    for (index, byte) in data.iter().take(8).enumerate() {
        wide_bytes[index] = *byte;
    }
    let wide_value = u64::from_le_bytes(wide_bytes);
    let _ = writer.write_u48_le(wide_value);
    let _ = writer.write_u48_be(wide_value);
    writer.write_u64_le(wide_value);
    writer.write_u64_be(wide_value);
    writer.write_i64_le(wide_value as i64);
    writer.write_i64_be(wide_value as i64);
}
