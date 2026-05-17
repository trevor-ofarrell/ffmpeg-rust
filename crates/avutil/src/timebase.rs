use crate::{AvError, AvResult, Rational};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rounding {
    Zero,
    Inf,
    Down,
    Up,
    NearInf,
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
    rescale_q_rnd_inner(value, src, dst, rounding)
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

    rescale_q_rnd_inner(value, src, dst, rounding)
}

fn rescale_q_rnd_inner(
    value: i64,
    src: Rational,
    dst: Rational,
    rounding: Rounding,
) -> AvResult<i64> {
    if src.den() == 0 || dst.num() == 0 || dst.den() == 0 {
        return Err(AvError::invalid_argument("invalid time base for rescale"));
    }

    let numerator = i128::from(value) * i128::from(src.num()) * i128::from(dst.den());
    let denominator = i128::from(src.den()) * i128::from(dst.num());

    if denominator == 0 {
        return Err(AvError::invalid_argument(
            "invalid zero denominator for rescale",
        ));
    }

    let result = div_round(numerator, denominator, rounding);
    i64::try_from(result).map_err(|_| AvError::invalid_argument("rescaled timestamp out of range"))
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
    fn rescales_between_common_media_timebases() {
        let ninety_khz = Rational::new(1, 90_000).unwrap();
        let milliseconds = Rational::new(1, 1_000).unwrap();

        assert_eq!(rescale_q(90_000, ninety_khz, milliseconds).unwrap(), 1_000);
        assert_eq!(rescale_q(3_003, ninety_khz, milliseconds).unwrap(), 33);
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
