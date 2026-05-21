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
