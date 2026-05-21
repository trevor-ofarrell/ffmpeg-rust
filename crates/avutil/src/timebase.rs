use crate::{AvError, AvResult, Rational};
use core::cmp::Ordering;

pub const AV_TIME_BASE: i64 = 1_000_000;
pub const AV_TIME_BASE_Q: Rational = Rational::from_raw(1, AV_TIME_BASE as i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rounding {
    Zero,
    Inf,
    Down,
    Up,
    NearInf,
}

pub fn rescale(value: i64, multiplier: i64, divisor: i64) -> AvResult<i64> {
    rescale_rnd(value, multiplier, divisor, Rounding::NearInf)
}

pub fn rescale_rnd(value: i64, multiplier: i64, divisor: i64, rounding: Rounding) -> AvResult<i64> {
    rescale_rnd_inner(value, multiplier, divisor, rounding)
}

pub fn rescale_rnd_pass_minmax(
    value: i64,
    multiplier: i64,
    divisor: i64,
    rounding: Rounding,
) -> AvResult<i64> {
    if value == i64::MIN || value == i64::MAX {
        return Ok(value);
    }

    rescale_rnd_inner(value, multiplier, divisor, rounding)
}

pub fn rescale_q(value: i64, src: Rational, dst: Rational) -> AvResult<i64> {
    rescale_q_rnd(value, src, dst, Rounding::NearInf)
}

pub fn rescale_q_rnd(
    value: i64,
    src: Rational,
    dst: Rational,
    rounding: Rounding,
) -> AvResult<i64> {
    let (multiplier, divisor) = rescale_q_terms(src, dst)?;
    rescale_rnd_inner(value, multiplier, divisor, rounding)
}

pub fn rescale_q_rnd_pass_minmax(
    value: i64,
    src: Rational,
    dst: Rational,
    rounding: Rounding,
) -> AvResult<i64> {
    if value == i64::MIN || value == i64::MAX {
        return Ok(value);
    }

    let (multiplier, divisor) = rescale_q_terms(src, dst)?;
    rescale_rnd_inner(value, multiplier, divisor, rounding)
}

pub fn compare_ts(ts_a: i64, tb_a: Rational, ts_b: i64, tb_b: Rational) -> AvResult<Ordering> {
    ensure_positive_time_base(tb_a, "first timestamp time base")?;
    ensure_positive_time_base(tb_b, "second timestamp time base")?;

    let lhs = i128::from(ts_a) * i128::from(tb_a.num()) * i128::from(tb_b.den());
    let rhs = i128::from(ts_b) * i128::from(tb_b.num()) * i128::from(tb_a.den());
    Ok(lhs.cmp(&rhs))
}

pub fn compare_mod(a: u64, b: u64, modulus: u64) -> AvResult<i64> {
    if !modulus.is_power_of_two() {
        return Err(AvError::invalid_argument(
            "timestamp comparison modulus must be a nonzero power of two",
        ));
    }

    let diff = a.wrapping_sub(b) & (modulus - 1);
    let centered = if diff > (modulus >> 1) {
        i128::from(diff) - i128::from(modulus)
    } else {
        i128::from(diff)
    };
    i64::try_from(centered)
        .map_err(|_| AvError::invalid_argument("modular timestamp difference out of range"))
}

pub fn rescale_delta(
    in_tb: Rational,
    in_ts: i64,
    fs_tb: Rational,
    duration: i64,
    last: &mut i64,
    out_tb: Rational,
) -> AvResult<i64> {
    ensure_positive_time_base(in_tb, "input timestamp time base")?;
    ensure_positive_time_base(fs_tb, "frame/sample time base")?;
    ensure_positive_time_base(out_tb, "output timestamp time base")?;
    if in_ts == i64::MIN {
        return Err(AvError::invalid_argument(
            "rescale delta input timestamp must not be AV_NOPTS_VALUE",
        ));
    }
    if duration < 0 || duration > i64::from(i32::MAX) {
        return Err(AvError::invalid_argument(
            "rescale delta duration must be a nonnegative FFmpeg int",
        ));
    }

    let (output, next_last) =
        rescale_delta_with_last(in_tb, in_ts, fs_tb, duration, *last, out_tb)?;
    *last = next_last;
    Ok(output)
}

pub fn add_stable(ts_tb: Rational, ts: i64, inc_tb: Rational, inc: i64) -> AvResult<i64> {
    ensure_positive_time_base(ts_tb, "timestamp time base")?;
    ensure_positive_time_base(inc_tb, "increment time base")?;

    let scaled_inc = scaled_increment_time_base(inc_tb, inc)?;
    let m = i128::from(scaled_inc.num()) * i128::from(ts_tb.den());
    let d = i128::from(scaled_inc.den()) * i128::from(ts_tb.num());
    if d <= 0 {
        return Err(AvError::invalid_argument(
            "invalid stable timestamp time base ratio",
        ));
    }

    if m % d == 0 {
        let delta = i64::try_from(m / d)
            .map_err(|_| AvError::invalid_argument("stable timestamp increment out of range"))?;
        return ts
            .checked_add(delta)
            .ok_or_else(|| AvError::invalid_argument("stable timestamp result out of range"));
    }
    if m < d {
        return Ok(ts);
    }

    let old = rescale_q(ts, ts_tb, scaled_inc)?;
    let old_ts = rescale_q(old, scaled_inc, ts_tb)?;
    if old == i64::MAX || old == i64::MIN || old_ts == i64::MIN {
        return Ok(ts);
    }

    let next = old
        .checked_add(1)
        .ok_or_else(|| AvError::invalid_argument("stable timestamp increment overflow"))?;
    let next_ts = rescale_q(next, scaled_inc, ts_tb)?;
    let residual = ts
        .checked_sub(old_ts)
        .ok_or_else(|| AvError::invalid_argument("stable timestamp residual out of range"))?;
    Ok(next_ts.saturating_add(residual))
}

fn rescale_delta_with_last(
    in_tb: Rational,
    in_ts: i64,
    fs_tb: Rational,
    duration: i64,
    last: i64,
    out_tb: Rational,
) -> AvResult<(i64, i64)> {
    if rescale_delta_uses_simple_round(in_tb, duration, last, out_tb) {
        return rescale_delta_simple_round(in_tb, in_ts, fs_tb, duration, out_tb);
    }

    let start = in_ts
        .checked_mul(2)
        .and_then(|ts| ts.checked_sub(1))
        .ok_or_else(|| AvError::invalid_argument("rescale delta timestamp out of range"))?;
    let end = in_ts
        .checked_mul(2)
        .and_then(|ts| ts.checked_add(1))
        .ok_or_else(|| AvError::invalid_argument("rescale delta timestamp out of range"))?;
    let a = rescale_q_rnd(start, in_tb, fs_tb, Rounding::Down)? >> 1;
    let b = rescale_q_rnd(end, in_tb, fs_tb, Rounding::Up)?
        .checked_add(1)
        .ok_or_else(|| AvError::invalid_argument("rescale delta bounds out of range"))?
        >> 1;
    if a > b {
        return Err(AvError::invalid_argument(
            "rescale delta timestamp bounds are inconsistent",
        ));
    }

    let lower = 2_i128 * i128::from(a) - i128::from(b);
    let upper = 2_i128 * i128::from(b) - i128::from(a);
    let last_wide = i128::from(last);
    if last_wide < lower || last_wide > upper {
        return rescale_delta_simple_round(in_tb, in_ts, fs_tb, duration, out_tb);
    }

    let this = last.clamp(a, b);
    let next_last = this
        .checked_add(duration)
        .ok_or_else(|| AvError::invalid_argument("rescale delta duration out of range"))?;
    let output = rescale_q(this, fs_tb, out_tb)?;
    Ok((output, next_last))
}

fn rescale_delta_uses_simple_round(
    in_tb: Rational,
    duration: i64,
    last: i64,
    out_tb: Rational,
) -> bool {
    last == i64::MIN
        || duration == 0
        || i128::from(in_tb.num()) * i128::from(out_tb.den())
            <= i128::from(out_tb.num()) * i128::from(in_tb.den())
}

fn rescale_delta_simple_round(
    in_tb: Rational,
    in_ts: i64,
    fs_tb: Rational,
    duration: i64,
    out_tb: Rational,
) -> AvResult<(i64, i64)> {
    let scaled = rescale_q(in_ts, in_tb, fs_tb)?;
    let next_last = scaled
        .checked_add(duration)
        .ok_or_else(|| AvError::invalid_argument("rescale delta duration out of range"))?;
    let output = rescale_q(in_ts, in_tb, out_tb)?;
    Ok((output, next_last))
}

fn rescale_q_terms(src: Rational, dst: Rational) -> AvResult<(i64, i64)> {
    if src.num() < 0 || src.den() <= 0 || dst.num() <= 0 || dst.den() <= 0 {
        return Err(AvError::invalid_argument("invalid time base for rescale"));
    }

    let multiplier = i64::from(src.num()) * i64::from(dst.den());
    let divisor = i64::from(src.den()) * i64::from(dst.num());
    Ok((multiplier, divisor))
}

fn rescale_rnd_inner(
    value: i64,
    multiplier: i64,
    divisor: i64,
    rounding: Rounding,
) -> AvResult<i64> {
    if multiplier < 0 || divisor <= 0 {
        return Err(AvError::invalid_argument(
            "invalid multiplier or divisor for rescale",
        ));
    }

    let numerator = i128::from(value) * i128::from(multiplier);
    let result = div_round(numerator, i128::from(divisor), rounding);
    i64::try_from(result).map_err(|_| AvError::invalid_argument("rescaled timestamp out of range"))
}

fn ensure_positive_time_base(time_base: Rational, context: &str) -> AvResult<()> {
    if time_base.num() <= 0 || time_base.den() <= 0 {
        return Err(AvError::invalid_argument(format!(
            "{context} must be a positive rational"
        )));
    }
    Ok(())
}

fn scaled_increment_time_base(inc_tb: Rational, inc: i64) -> AvResult<Rational> {
    if inc == 1 {
        return Ok(inc_tb);
    }

    let num = i64::from(inc_tb.num())
        .checked_mul(inc)
        .ok_or_else(|| AvError::invalid_argument("stable timestamp increment out of range"))?;
    Rational::reduce_i64(num, i64::from(inc_tb.den()), i32::MAX).map(|(rational, _)| rational)
}

fn div_round(numerator: i128, denominator: i128, rounding: Rounding) -> i128 {
    debug_assert_ne!(denominator, 0);

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
            let abs_rem = remainder.abs();
            let abs_den = denominator.abs();
            if abs_rem * 2 >= abs_den {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_ffmpeg_timebase_constants() {
        assert_eq!(AV_TIME_BASE, 1_000_000);
        assert_eq!(AV_TIME_BASE_Q, Rational::new(1, 1_000_000).unwrap());
    }

    #[test]
    fn rescales_integer_terms_with_rounding_modes() {
        assert_eq!(rescale(3, 1, 2).unwrap(), 2);
        assert_eq!(rescale_rnd(1, 1, 3, Rounding::Zero).unwrap(), 0);
        assert_eq!(rescale_rnd(1, 1, 3, Rounding::Inf).unwrap(), 1);
        assert_eq!(rescale_rnd(-1, 1, 3, Rounding::Inf).unwrap(), -1);
        assert_eq!(rescale_rnd(1, 1, 3, Rounding::Up).unwrap(), 1);
        assert_eq!(rescale_rnd(-1, 1, 3, Rounding::Up).unwrap(), 0);
        assert_eq!(rescale_rnd(-1, 1, 3, Rounding::Down).unwrap(), -1);
        assert_eq!(rescale_rnd(1, 1, 2, Rounding::NearInf).unwrap(), 1);
        assert_eq!(rescale_rnd(-1, 1, 2, Rounding::NearInf).unwrap(), -1);
    }

    #[test]
    fn direct_pass_minmax_preserves_timestamp_sentinels() {
        assert_eq!(
            rescale_rnd_pass_minmax(i64::MIN, 1, 2, Rounding::Up).unwrap(),
            i64::MIN
        );
        assert_eq!(
            rescale_rnd_pass_minmax(i64::MAX, 1, 2, Rounding::Up).unwrap(),
            i64::MAX
        );
        assert_eq!(rescale_rnd_pass_minmax(3, 1, 2, Rounding::Up).unwrap(), 2);
    }

    #[test]
    fn direct_rescale_rejects_invalid_terms_and_overflow() {
        assert_eq!(
            rescale_rnd(1, -1, 2, Rounding::NearInf).unwrap_err().kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(
            rescale_rnd(1, 1, 0, Rounding::NearInf).unwrap_err().kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(
            rescale_rnd(i64::MAX, i64::MAX, 1, Rounding::NearInf)
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidArgument
        );
    }

    #[test]
    fn rescales_between_common_media_timebases() {
        let ninety_khz = Rational::new(1, 90_000).unwrap();
        let milliseconds = Rational::new(1, 1_000).unwrap();

        assert_eq!(rescale_q(90_000, ninety_khz, milliseconds).unwrap(), 1_000);
        assert_eq!(rescale_q(3_003, ninety_khz, milliseconds).unwrap(), 33);
    }

    #[test]
    fn compares_timestamps_across_timebases() {
        let milliseconds = Rational::new(1, 1_000).unwrap();
        let seconds = Rational::ONE;
        let ninety_khz = Rational::new(1, 90_000).unwrap();

        assert_eq!(
            compare_ts(1_000, milliseconds, 1, seconds).unwrap(),
            Ordering::Equal
        );
        assert_eq!(
            compare_ts(3_003, ninety_khz, 33, milliseconds).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            compare_ts(-500, milliseconds, 0, seconds).unwrap(),
            Ordering::Less
        );
        assert_eq!(
            compare_ts(
                i64::MAX / 4,
                Rational::new(1, 3).unwrap(),
                i64::MAX / 2,
                Rational::new(1, 2).unwrap()
            )
            .unwrap(),
            Ordering::Less
        );
    }

    #[test]
    fn compare_ts_rejects_invalid_timebases() {
        assert_eq!(
            compare_ts(1, Rational::from_raw(0, 1), 1, Rational::ONE)
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(
            compare_ts(1, Rational::ONE, 1, Rational::from_raw(1, -1))
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidArgument
        );
    }

    #[test]
    fn compares_power_of_two_modular_timestamps() {
        assert_eq!(compare_mod(0x11, 0x02, 0x10).unwrap(), -1);
        assert_eq!(compare_mod(0x11, 0x02, 0x20).unwrap(), 15);
        assert_eq!(compare_mod(0x02, 0x11, 0x10).unwrap(), 1);
        assert_eq!(compare_mod(0x12, 0x02, 0x10).unwrap(), 0);
        assert_eq!(compare_mod(u64::MAX, 0, 1 << 4).unwrap(), -1);
        assert_eq!(compare_mod(0, 0, 1).unwrap(), 0);
    }

    #[test]
    fn compare_mod_rejects_invalid_moduli() {
        assert_eq!(
            compare_mod(1, 0, 0).unwrap_err().kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(
            compare_mod(1, 0, 3).unwrap_err().kind(),
            crate::AvErrorKind::InvalidArgument
        );
    }

    #[test]
    fn rescale_delta_initializes_last_on_first_call() {
        let milliseconds = Rational::new(1, 1_000).unwrap();
        let samples_48k = Rational::new(1, 48_000).unwrap();
        let ninety_khz = Rational::new(1, 90_000).unwrap();
        let mut last = i64::MIN;

        assert_eq!(
            rescale_delta(milliseconds, 100, samples_48k, 1_024, &mut last, ninety_khz).unwrap(),
            9_000
        );
        assert_eq!(last, 5_824);
    }

    #[test]
    fn rescale_delta_zero_duration_uses_simple_rounding() {
        let milliseconds = Rational::new(1, 1_000).unwrap();
        let samples_48k = Rational::new(1, 48_000).unwrap();
        let ninety_khz = Rational::new(1, 90_000).unwrap();
        let mut last = 123;

        assert_eq!(
            rescale_delta(milliseconds, 250, samples_48k, 0, &mut last, ninety_khz).unwrap(),
            22_500
        );
        assert_eq!(last, 12_000);
    }

    #[test]
    fn rescale_delta_stateful_path_preserves_known_duration() {
        let milliseconds = Rational::new(1, 1_000).unwrap();
        let samples_48k = Rational::new(1, 48_000).unwrap();
        let mut last = 48_010;

        assert_eq!(
            rescale_delta(
                milliseconds,
                1_000,
                samples_48k,
                1_024,
                &mut last,
                samples_48k
            )
            .unwrap(),
            48_010
        );
        assert_eq!(last, 49_034);
    }

    #[test]
    fn rescale_delta_clips_last_inside_input_window() {
        let milliseconds = Rational::new(1, 1_000).unwrap();
        let samples_48k = Rational::new(1, 48_000).unwrap();
        let mut last = 48_050;

        assert_eq!(
            rescale_delta(
                milliseconds,
                1_000,
                samples_48k,
                1_024,
                &mut last,
                samples_48k
            )
            .unwrap(),
            48_024
        );
        assert_eq!(last, 49_048);
    }

    #[test]
    fn rescale_delta_out_of_window_falls_back_to_input_timestamp() {
        let milliseconds = Rational::new(1, 1_000).unwrap();
        let samples_48k = Rational::new(1, 48_000).unwrap();
        let mut last = 47_000;

        assert_eq!(
            rescale_delta(
                milliseconds,
                1_000,
                samples_48k,
                1_024,
                &mut last,
                samples_48k
            )
            .unwrap(),
            48_000
        );
        assert_eq!(last, 49_024);
    }

    #[test]
    fn rescale_delta_rejects_invalid_inputs_without_mutating_last() {
        let milliseconds = Rational::new(1, 1_000).unwrap();
        let samples_48k = Rational::new(1, 48_000).unwrap();
        let mut last = 321;

        assert_eq!(
            rescale_delta(
                Rational::from_raw(0, 1),
                1,
                samples_48k,
                1,
                &mut last,
                samples_48k
            )
            .unwrap_err()
            .kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(last, 321);
        assert_eq!(
            rescale_delta(
                milliseconds,
                i64::MIN,
                samples_48k,
                1,
                &mut last,
                samples_48k
            )
            .unwrap_err()
            .kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(last, 321);
        assert_eq!(
            rescale_delta(
                milliseconds,
                1,
                Rational::from_raw(1, 0),
                1,
                &mut last,
                samples_48k
            )
            .unwrap_err()
            .kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(last, 321);
        assert_eq!(
            rescale_delta(milliseconds, 1, samples_48k, -1, &mut last, samples_48k)
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(last, 321);
        assert_eq!(
            rescale_delta(
                milliseconds,
                1,
                samples_48k,
                i64::from(i32::MAX) + 1,
                &mut last,
                samples_48k
            )
            .unwrap_err()
            .kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(last, 321);
        assert_eq!(
            rescale_delta(
                milliseconds,
                i64::MAX,
                samples_48k,
                1,
                &mut last,
                samples_48k
            )
            .unwrap_err()
            .kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(last, 321);
    }

    #[test]
    fn add_stable_adds_exact_tick_increments() {
        let milliseconds = Rational::new(1, 1_000).unwrap();

        assert_eq!(
            add_stable(milliseconds, 1_000, milliseconds, 40).unwrap(),
            1_040
        );
        assert_eq!(add_stable(milliseconds, -10, milliseconds, 10).unwrap(), 0);
        assert_eq!(add_stable(milliseconds, 123, milliseconds, 0).unwrap(), 123);
    }

    #[test]
    fn add_stable_subtracts_exact_negative_tick_increments() {
        let milliseconds = Rational::new(1, 1_000).unwrap();

        assert_eq!(
            add_stable(milliseconds, 1_000, milliseconds, -40).unwrap(),
            960
        );
        assert_eq!(add_stable(milliseconds, 10, milliseconds, -10).unwrap(), 0);
    }

    #[test]
    fn add_stable_avoids_repeated_fractional_drift() {
        let milliseconds = Rational::new(1, 1_000).unwrap();
        let thirtieth = Rational::new(1, 30).unwrap();
        let mut ts = 0;
        let expected = [33, 67, 100, 133, 167, 200, 233, 267, 300, 333];

        for expected_ts in expected {
            ts = add_stable(milliseconds, ts, thirtieth, 1).unwrap();
            assert_eq!(ts, expected_ts);
        }
    }

    #[test]
    fn add_stable_preserves_existing_fractional_phase() {
        let milliseconds = Rational::new(1, 1_000).unwrap();
        let thirtieth = Rational::new(1, 30).unwrap();

        assert_eq!(add_stable(milliseconds, 1, thirtieth, 1).unwrap(), 34);
        assert_eq!(add_stable(milliseconds, 34, thirtieth, 1).unwrap(), 68);
    }

    #[test]
    fn add_stable_keeps_sub_tick_increments_unchanged() {
        let milliseconds = Rational::new(1, 1_000).unwrap();
        let samples_48k = Rational::new(1, 48_000).unwrap();

        assert_eq!(add_stable(milliseconds, 123, samples_48k, 1).unwrap(), 123);
    }

    #[test]
    fn add_stable_keeps_fractional_negative_increments_unchanged() {
        let milliseconds = Rational::new(1, 1_000).unwrap();
        let thirtieth = Rational::new(1, 30).unwrap();
        let samples_48k = Rational::new(1, 48_000).unwrap();

        assert_eq!(
            add_stable(milliseconds, 1_000, thirtieth, -1).unwrap(),
            1_000
        );
        assert_eq!(add_stable(milliseconds, 123, samples_48k, -1).unwrap(), 123);
    }

    #[test]
    fn add_stable_rejects_invalid_inputs() {
        let milliseconds = Rational::new(1, 1_000).unwrap();

        assert_eq!(
            add_stable(Rational::from_raw(0, 1), 0, milliseconds, 1)
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(
            add_stable(milliseconds, 0, Rational::from_raw(1, 0), 1)
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(
            add_stable(milliseconds, i64::MAX, milliseconds, 1)
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(
            add_stable(milliseconds, i64::MIN, milliseconds, -1)
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidArgument
        );
    }

    #[test]
    fn supports_explicit_rounding_modes() {
        let src = Rational::new(1, 3).unwrap();
        let dst = Rational::new(1, 1).unwrap();

        assert_eq!(rescale_q_rnd(1, src, dst, Rounding::Zero).unwrap(), 0);
        assert_eq!(rescale_q_rnd(1, src, dst, Rounding::Inf).unwrap(), 1);
        assert_eq!(rescale_q_rnd(-1, src, dst, Rounding::Inf).unwrap(), -1);
        assert_eq!(rescale_q_rnd(1, src, dst, Rounding::Up).unwrap(), 1);
        assert_eq!(rescale_q_rnd(-1, src, dst, Rounding::Up).unwrap(), 0);
        assert_eq!(rescale_q_rnd(-1, src, dst, Rounding::Down).unwrap(), -1);
    }

    #[test]
    fn pass_minmax_preserves_timestamp_sentinels() {
        let src = Rational::new(1, 1_000).unwrap();
        let dst = Rational::new(1, 1).unwrap();

        assert_eq!(
            rescale_q_rnd_pass_minmax(i64::MIN, src, dst, Rounding::NearInf).unwrap(),
            i64::MIN
        );
        assert_eq!(
            rescale_q_rnd_pass_minmax(i64::MAX, src, dst, Rounding::NearInf).unwrap(),
            i64::MAX
        );
        assert_eq!(
            rescale_q_rnd_pass_minmax(1_500, src, dst, Rounding::NearInf).unwrap(),
            2
        );
    }

    #[test]
    fn rejects_invalid_time_bases_and_out_of_range_results() {
        let one = Rational::ONE;

        assert_eq!(
            rescale_q_rnd(1, Rational::from_raw(1, 0), one, Rounding::NearInf)
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(
            rescale_q_rnd(1, Rational::from_raw(-1, 1), one, Rounding::NearInf)
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(
            rescale_q_rnd(1, one, Rational::from_raw(1, -1), Rounding::NearInf)
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(
            rescale_q_rnd(1, one, Rational::from_raw(0, 1), Rounding::NearInf)
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(
            rescale_q_rnd(
                i64::MAX,
                one,
                Rational::new(1, 2).unwrap(),
                Rounding::NearInf
            )
            .unwrap_err()
            .kind(),
            crate::AvErrorKind::InvalidArgument
        );
    }
}
