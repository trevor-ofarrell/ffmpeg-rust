#![no_main]

use avutil::{AvResult, ByteReader, ByteWriter};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let mut reader = ByteReader::new(data);
    let mut control = ByteReader::new(data);
    let mut writer = ByteWriter::new();

    while let Ok(op) = control.read_u8() {
        let before = reader.position();
        let opcode = op % 54;
        let is_peek = (24..=46).contains(&opcode) || matches!(opcode, 51 | 53);
        let may_move_backward = matches!(opcode, 47..=49);
        let result = run_read_operation(&mut reader, opcode, op);

        assert!(reader.position() <= reader.len());
        if result.is_err() || is_peek {
            assert_eq!(reader.position(), before);
        } else if !may_move_backward {
            assert!(reader.position() >= before);
        }

        run_write_operation(&mut writer, data, op);
    }

    assert!(reader.remaining() <= reader.len());
    assert_eq!(reader.is_eof(), reader.remaining() == 0);
    assert_eq!(writer.is_empty(), writer.as_slice().is_empty());
});

fn run_read_operation(reader: &mut ByteReader<'_>, opcode: u8, op: u8) -> AvResult<()> {
    match opcode {
        0 => reader.read_u8().map(|_| ()),
        1 => reader.read_i8().map(|_| ()),
        2 => reader.read_u16_le().map(|_| ()),
        3 => reader.read_u16_be().map(|_| ()),
        4 => reader.read_i16_le().map(|_| ()),
        5 => reader.read_i16_be().map(|_| ()),
        6 => reader.read_u24_le().map(|_| ()),
        7 => reader.read_u24_be().map(|_| ()),
        8 => reader.read_i24_le().map(|_| ()),
        9 => reader.read_i24_be().map(|_| ()),
        10 => reader.read_u32_le().map(|_| ()),
        11 => reader.read_u32_be().map(|_| ()),
        12 => reader.read_i32_le().map(|_| ()),
        13 => reader.read_i32_be().map(|_| ()),
        14 => reader.read_u48_le().map(|_| ()),
        15 => reader.read_u48_be().map(|_| ()),
        16 => reader.read_i48_le().map(|_| ()),
        17 => reader.read_i48_be().map(|_| ()),
        18 => reader.read_u64_le().map(|_| ()),
        19 => reader.read_u64_be().map(|_| ()),
        20 => reader.read_i64_le().map(|_| ()),
        21 => reader.read_i64_be().map(|_| ()),
        22 => reader.skip(usize::from(op >> 4)),
        23 => reader.read_exact(usize::from(op >> 4)).map(|_| ()),
        24 => reader.peek_u8().map(|_| ()),
        25 => reader.peek_i8().map(|_| ()),
        26 => reader.peek_u16_le().map(|_| ()),
        27 => reader.peek_u16_be().map(|_| ()),
        28 => reader.peek_i16_le().map(|_| ()),
        29 => reader.peek_i16_be().map(|_| ()),
        30 => reader.peek_u24_le().map(|_| ()),
        31 => reader.peek_u24_be().map(|_| ()),
        32 => reader.peek_i24_le().map(|_| ()),
        33 => reader.peek_i24_be().map(|_| ()),
        34 => reader.peek_u32_le().map(|_| ()),
        35 => reader.peek_u32_be().map(|_| ()),
        36 => reader.peek_i32_le().map(|_| ()),
        37 => reader.peek_i32_be().map(|_| ()),
        38 => reader.peek_u48_le().map(|_| ()),
        39 => reader.peek_u48_be().map(|_| ()),
        40 => reader.peek_i48_le().map(|_| ()),
        41 => reader.peek_i48_be().map(|_| ()),
        42 => reader.peek_u64_le().map(|_| ()),
        43 => reader.peek_u64_be().map(|_| ()),
        44 => reader.peek_i64_le().map(|_| ()),
        45 => reader.peek_i64_be().map(|_| ()),
        46 => reader.peek_exact(usize::from(op >> 4)).map(|_| ()),
        47 => reader.set_position(usize::from(op)),
        48 => reader.seek_relative(i8::from_ne_bytes([op]) as isize),
        49 => {
            reader.rewind();
            Ok(())
        }
        50 => reader.read_array::<4>().map(|_| ()),
        51 => reader.peek_array::<4>().map(|_| ()),
        52 => reader.read_tag().map(|_| ()),
        _ => reader.peek_tag().map(|_| ()),
    }
}

fn run_write_operation(writer: &mut ByteWriter, data: &[u8], op: u8) {
    writer.write_u8(op);
    writer.write_i8(op as i8);
    writer.write_u16_le(u16::from(op));
    writer.write_u16_be(u16::from(op));
    writer.write_i16_le(i16::from(op));
    writer.write_i16_be(i16::from(op));
    writer.write_tag(tag_from_data(data, op));

    let mut bytes = [0_u8; 4];
    for (index, byte) in data.iter().take(4).enumerate() {
        bytes[index] = *byte;
    }
    let value = u32::from_le_bytes(bytes);
    let _ = writer.write_u24_le(value);
    let _ = writer.write_u24_be(value);
    write_i24_and_assert_non_mutation(writer, value as i32, true);
    write_i24_and_assert_non_mutation(writer, value as i32, false);
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
    write_i48_and_assert_non_mutation(writer, wide_value as i64, true);
    write_i48_and_assert_non_mutation(writer, wide_value as i64, false);
    writer.write_u64_le(wide_value);
    writer.write_u64_be(wide_value);
    writer.write_i64_le(wide_value as i64);
    writer.write_i64_be(wide_value as i64);

    run_patch_operations(writer, data, op, value, wide_value);
}

fn tag_from_data(data: &[u8], fallback: u8) -> [u8; 4] {
    let mut tag = [fallback; 4];
    for (index, byte) in data.iter().take(4).enumerate() {
        tag[index] = *byte;
    }
    tag
}

fn write_i24_and_assert_non_mutation(writer: &mut ByteWriter, value: i32, little_endian: bool) {
    let before = writer.as_slice().to_vec();
    let result = if little_endian {
        writer.write_i24_le(value)
    } else {
        writer.write_i24_be(value)
    };

    if result.is_err() {
        assert_eq!(writer.as_slice(), before.as_slice());
    }
}

fn write_i48_and_assert_non_mutation(writer: &mut ByteWriter, value: i64, little_endian: bool) {
    let before = writer.as_slice().to_vec();
    let result = if little_endian {
        writer.write_i48_le(value)
    } else {
        writer.write_i48_be(value)
    };

    if result.is_err() {
        assert_eq!(writer.as_slice(), before.as_slice());
    }
}

fn run_patch_operations(writer: &mut ByteWriter, data: &[u8], op: u8, value: u32, wide_value: u64) {
    let offset = usize::from(op);
    let patch_len = data.len().min(4);
    let before_patch_all = writer.as_slice().to_vec();
    let patch_all_result = writer.patch_all(offset, &data[..patch_len]);
    if patch_all_result.is_err() {
        assert_eq!(writer.as_slice(), before_patch_all.as_slice());
    }

    patch_and_assert_non_mutation(writer, offset, |writer| writer.patch_u8(offset, op));
    patch_and_assert_non_mutation(writer, offset, |writer| writer.patch_i8(offset, op as i8));
    patch_and_assert_non_mutation(writer, offset, |writer| {
        writer.patch_tag(offset, tag_from_data(data, op))
    });
    patch_and_assert_non_mutation(writer, offset, |writer| {
        writer.patch_u16_le(offset, u16::from(op))
    });
    patch_and_assert_non_mutation(writer, offset, |writer| {
        writer.patch_u16_be(offset, u16::from(op))
    });
    patch_and_assert_non_mutation(writer, offset, |writer| {
        writer.patch_i16_le(offset, i16::from(op))
    });
    patch_and_assert_non_mutation(writer, offset, |writer| {
        writer.patch_i16_be(offset, i16::from(op))
    });
    patch_and_assert_non_mutation(writer, offset, |writer| writer.patch_u24_le(offset, value));
    patch_and_assert_non_mutation(writer, offset, |writer| writer.patch_u24_be(offset, value));
    patch_and_assert_non_mutation(writer, offset, |writer| {
        writer.patch_i24_le(offset, value as i32)
    });
    patch_and_assert_non_mutation(writer, offset, |writer| {
        writer.patch_i24_be(offset, value as i32)
    });
    patch_and_assert_non_mutation(writer, offset, |writer| writer.patch_u32_le(offset, value));
    patch_and_assert_non_mutation(writer, offset, |writer| writer.patch_u32_be(offset, value));
    patch_and_assert_non_mutation(writer, offset, |writer| {
        writer.patch_i32_le(offset, value as i32)
    });
    patch_and_assert_non_mutation(writer, offset, |writer| {
        writer.patch_i32_be(offset, value as i32)
    });
    patch_and_assert_non_mutation(writer, offset, |writer| {
        writer.patch_u48_le(offset, wide_value)
    });
    patch_and_assert_non_mutation(writer, offset, |writer| {
        writer.patch_u48_be(offset, wide_value)
    });
    patch_and_assert_non_mutation(writer, offset, |writer| {
        writer.patch_i48_le(offset, wide_value as i64)
    });
    patch_and_assert_non_mutation(writer, offset, |writer| {
        writer.patch_i48_be(offset, wide_value as i64)
    });
    patch_and_assert_non_mutation(writer, offset, |writer| {
        writer.patch_u64_le(offset, wide_value)
    });
    patch_and_assert_non_mutation(writer, offset, |writer| {
        writer.patch_u64_be(offset, wide_value)
    });
    patch_and_assert_non_mutation(writer, offset, |writer| {
        writer.patch_i64_le(offset, wide_value as i64)
    });
    patch_and_assert_non_mutation(writer, offset, |writer| {
        writer.patch_i64_be(offset, wide_value as i64)
    });

    let before_truncate = writer.as_slice().to_vec();
    let truncate_result = writer.truncate(offset);
    if truncate_result.is_err() {
        assert_eq!(writer.as_slice(), before_truncate.as_slice());
    } else {
        assert_eq!(writer.len(), offset);
        assert_eq!(writer.position(), offset);
    }
}

fn patch_and_assert_non_mutation(
    writer: &mut ByteWriter,
    offset: usize,
    patch: impl FnOnce(&mut ByteWriter) -> AvResult<()>,
) {
    let before = writer.as_slice().to_vec();
    let result = patch(writer);
    if result.is_err() {
        assert_eq!(writer.as_slice(), before.as_slice());
    } else {
        assert!(writer.len() >= offset);
    }
}
