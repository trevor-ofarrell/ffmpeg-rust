use crate::{AvError, AvResult};
use core::cmp::Ordering;
use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rational {
    num: i32,
    den: i32,
}

impl Rational {
    pub const ZERO: Self = Self { num: 0, den: 1 };
    pub const ONE: Self = Self { num: 1, den: 1 };

    pub fn new(num: i32, den: i32) -> AvResult<Self> {
        if den == 0 {
            return Err(AvError::invalid_argument(
                "rational denominator must not be zero",
            ));
        }

        if num == 0 {
            return Ok(Self::ZERO);
        }

        let mut num = num;
        let mut den = den;
        if den < 0 {
            num = num.checked_neg().ok_or_else(|| {
                AvError::invalid_argument("rational numerator overflow while normalizing")
            })?;
            den = den.checked_neg().ok_or_else(|| {
                AvError::invalid_argument("rational denominator overflow while normalizing")
            })?;
        }

        let divisor = gcd_i32(num, den);
        Ok(Self {
            num: num / divisor,
            den: den / divisor,
        })
    }

    pub const fn from_raw(num: i32, den: i32) -> Self {
        Self { num, den }
    }

    pub fn num(self) -> i32 {
        self.num
    }

    pub fn den(self) -> i32 {
        self.den
    }

    pub fn to_f64(self) -> f64 {
        f64::from(self.num) / f64::from(self.den)
    }

    pub fn av_cmp(self, other: Self) -> Option<Ordering> {
        let diff = i128::from(self.num) * i128::from(other.den)
            - i128::from(other.num) * i128::from(self.den);

        if diff != 0 {
            let denominators_flip_sign = (self.den < 0) ^ (other.den < 0);
            let self_is_greater = if denominators_flip_sign {
                diff < 0
            } else {
                diff > 0
            };
            return Some(if self_is_greater {
                Ordering::Greater
            } else {
                Ordering::Less
            });
        }

        if self.den != 0 && other.den != 0 {
            Some(Ordering::Equal)
        } else if self.num != 0 && other.num != 0 {
            Some(self.num.signum().cmp(&other.num.signum()))
        } else {
            None
        }
    }

    pub fn nearer_to(self, first: Self, second: Self) -> AvResult<Ordering> {
        self.ensure_finite("reference rational for nearest comparison")?;
        first.ensure_finite("first rational for nearest comparison")?;
        second.ensure_finite("second rational for nearest comparison")?;

        let reference_num = i128::from(self.num);
        let reference_den = i128::from(self.den);
        let first_num = i128::from(first.num);
        let first_den = i128::from(first.den);
        let second_num = i128::from(second.num);
        let second_den = i128::from(second.den);

        let first_distance_num = (reference_num * first_den - first_num * reference_den).abs();
        let second_distance_num = (reference_num * second_den - second_num * reference_den).abs();
        let first_scaled = first_distance_num * second_den.abs();
        let second_scaled = second_distance_num * first_den.abs();

        Ok(match first_scaled.cmp(&second_scaled) {
            Ordering::Less => Ordering::Greater,
            Ordering::Equal => Ordering::Equal,
            Ordering::Greater => Ordering::Less,
        })
    }

    pub fn find_nearest_index(self, candidates: &[Self]) -> AvResult<Option<usize>> {
        let Some((first, rest)) = candidates.split_first() else {
            return Ok(None);
        };
        first.ensure_finite("first nearest rational candidate")?;

        let mut nearest = 0;
        for (offset, candidate) in rest.iter().enumerate() {
            let index = offset + 1;
            if self.nearer_to(*candidate, candidates[nearest])? == Ordering::Greater {
                nearest = index;
            }
        }

        Ok(Some(nearest))
    }

    pub fn gcd_with_limit(self, other: Self, max_den: i32, default: Self) -> AvResult<Self> {
        if max_den <= 0 {
            return Err(AvError::invalid_argument(
                "rational gcd maximum denominator must be positive",
            ));
        }
        self.ensure_positive_denominator("first rational for gcd")?;
        other.ensure_positive_denominator("second rational for gcd")?;

        let den_gcd = gcd_i128(i128::from(self.den), i128::from(other.den));
        let lcm = (i128::from(self.den) / den_gcd) * i128::from(other.den);
        if lcm >= i128::from(max_den) {
            return Ok(default);
        }

        let num_gcd = gcd_i128(i128::from(self.num), i128::from(other.num));
        let num = i32::try_from(num_gcd)
            .map_err(|_| AvError::invalid_argument("rational gcd numerator out of range"))?;
        let den = i32::try_from(lcm)
            .map_err(|_| AvError::invalid_argument("rational gcd denominator out of range"))?;
        Ok(Self::from_raw(num, den))
    }

    pub fn reduce_i64(num: i64, den: i64, max: i32) -> AvResult<(Self, bool)> {
        if max <= 0 {
            return Err(AvError::invalid_argument(
                "rational reduction maximum must be positive",
            ));
        }

        let (num, exact) = reduce_signed_i64(num, den, i128::from(max))?;
        Ok((num, exact))
    }

    pub fn from_f64_limited(value: f64, max: i32) -> AvResult<Self> {
        if max <= 0 {
            return Err(AvError::invalid_argument(
                "rational conversion maximum must be positive",
            ));
        }

        if value.is_nan() {
            return Ok(Self::from_raw(0, 0));
        }

        if value.abs() > f64::from(i32::MAX) + 3.0 {
            return Ok(Self::from_raw(
                if value.is_sign_negative() { -1 } else { 1 },
                0,
            ));
        }

        let exponent = if value == 0.0 {
            0
        } else {
            value.abs().log2().floor() as i32 + 1
        };
        let shift = 62 - (exponent - 1).max(0);
        let den = 1_i64
            .checked_shl(u32::try_from(shift).map_err(|_| {
                AvError::invalid_argument("rational conversion exponent out of range")
            })?)
            .ok_or_else(|| AvError::invalid_argument("rational conversion denominator overflow"))?;
        let num = (value * den as f64 + 0.5).floor() as i64;
        Self::reduce_i64(num, den, max).map(|(rational, _)| rational)
    }

    pub fn reciprocal(self) -> AvResult<Self> {
        if self.num == 0 {
            return Err(AvError::invalid_argument("cannot invert zero rational"));
        }

        Self::new(self.den, self.num)
    }

    pub fn checked_neg(self) -> AvResult<Self> {
        rational_from_i128(-i128::from(self.num), i128::from(self.den))
    }

    pub fn checked_add(self, other: Self) -> AvResult<Self> {
        let num = i128::from(self.num) * i128::from(other.den)
            + i128::from(other.num) * i128::from(self.den);
        let den = i128::from(self.den) * i128::from(other.den);
        rational_from_i128(num, den)
    }

    pub fn checked_sub(self, other: Self) -> AvResult<Self> {
        let num = i128::from(self.num) * i128::from(other.den)
            - i128::from(other.num) * i128::from(self.den);
        let den = i128::from(self.den) * i128::from(other.den);
        rational_from_i128(num, den)
    }

    pub fn checked_mul(self, other: Self) -> AvResult<Self> {
        let num = i128::from(self.num) * i128::from(other.num);
        let den = i128::from(self.den) * i128::from(other.den);
        rational_from_i128(num, den)
    }

    pub fn checked_div(self, other: Self) -> AvResult<Self> {
        self.checked_mul(other.reciprocal()?)
    }

    fn ensure_finite(self, context: &str) -> AvResult<()> {
        if self.den == 0 {
            return Err(AvError::invalid_argument(format!(
                "{context} must have a nonzero denominator"
            )));
        }
        Ok(())
    }

    fn ensure_positive_denominator(self, context: &str) -> AvResult<()> {
        if self.den <= 0 {
            return Err(AvError::invalid_argument(format!(
                "{context} must have a positive denominator"
            )));
        }
        Ok(())
    }
}

fn reduce_signed_i64(num: i64, den: i64, max: i128) -> AvResult<(Rational, bool)> {
    let negative = (num < 0) ^ (den < 0);
    let mut num = i128::from(num).abs();
    let mut den = i128::from(den).abs();
    let divisor = gcd_i128(num, den);
    if divisor > 0 {
        num /= divisor;
        den /= divisor;
    }

    let (reduced_num, reduced_den, exact) = reduce_abs_fraction(num, den, max);
    let signed_num = if negative { -reduced_num } else { reduced_num };
    let signed_num = i32::try_from(signed_num)
        .map_err(|_| AvError::invalid_argument("reduced rational numerator out of range"))?;
    let reduced_den = i32::try_from(reduced_den)
        .map_err(|_| AvError::invalid_argument("reduced rational denominator out of range"))?;
    Ok((Rational::from_raw(signed_num, reduced_den), exact))
}

fn reduce_abs_fraction(mut num: i128, mut den: i128, max: i128) -> (i128, i128, bool) {
    if num <= max && den <= max {
        return (num, den, true);
    }

    let original = (num, den);
    let mut previous = (0_i128, 1_i128);
    let mut current = (1_i128, 0_i128);

    while den != 0 {
        let quotient = num / den;
        let remainder = num - den * quotient;
        let next = (
            quotient * current.0 + previous.0,
            quotient * current.1 + previous.1,
        );

        if next.0 > max || next.1 > max {
            let scale_by_num = if current.0 == 0 {
                i128::MAX
            } else {
                (max - previous.0) / current.0
            };
            let scale_by_den = if current.1 == 0 {
                i128::MAX
            } else {
                (max - previous.1) / current.1
            };
            let scale = scale_by_num.min(scale_by_den).max(0);
            let bounded = (
                scale * current.0 + previous.0,
                scale * current.1 + previous.1,
            );
            if bounded.1 != 0 && is_better_reduction(bounded, current, original) {
                current = bounded;
            }
            return (current.0, current.1, false);
        }

        previous = current;
        current = next;
        num = den;
        den = remainder;
    }

    (current.0, current.1, true)
}

fn is_better_reduction(
    candidate: (i128, i128),
    current: (i128, i128),
    original: (i128, i128),
) -> bool {
    if current.1 == 0 {
        return candidate.1 != 0;
    }
    let (num, den) = original;
    let candidate_error = (num * candidate.1 - den * candidate.0).abs();
    let current_error = (num * current.1 - den * current.0).abs();
    candidate_error * current.1 < current_error * candidate.1
}

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.av_cmp(*other)
    }
}

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.num, self.den)
    }
}

fn rational_from_i128(num: i128, den: i128) -> AvResult<Rational> {
    if den == 0 {
        return Err(AvError::invalid_argument(
            "rational denominator must not be zero",
        ));
    }

    let divisor = gcd_i128(num, den);
    let num = num / divisor;
    let den = den / divisor;

    let num = i32::try_from(num)
        .map_err(|_| AvError::invalid_argument("rational numerator out of range"))?;
    let den = i32::try_from(den)
        .map_err(|_| AvError::invalid_argument("rational denominator out of range"))?;
    Rational::new(num, den)
}

fn gcd_i32(a: i32, b: i32) -> i32 {
    i32::try_from(gcd_i64(i64::from(a), i64::from(b))).expect("gcd of i32 values fits i32")
}

fn gcd_i64(a: i64, b: i64) -> i64 {
    let mut a = a.unsigned_abs();
    let mut b = b.unsigned_abs();

    while b != 0 {
        let rem = a % b;
        a = b;
        b = rem;
    }

    i64::try_from(a).unwrap_or(i64::MAX).max(1)
}

fn gcd_i128(a: i128, b: i128) -> i128 {
    let mut a = a.unsigned_abs();
    let mut b = b.unsigned_abs();

    while b != 0 {
        let rem = a % b;
        a = b;
        b = rem;
    }

    i128::try_from(a).unwrap_or(i128::MAX).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rational_normalizes_sign_and_gcd() {
        assert_eq!(
            Rational::new(30000, -1001).unwrap().to_string(),
            "-30000/1001"
        );
        assert_eq!(Rational::new(2, 4).unwrap(), Rational::from_raw(1, 2));
        assert_eq!(Rational::new(0, -400).unwrap(), Rational::ZERO);
    }

    #[test]
    fn rational_rejects_zero_denominator() {
        let err = Rational::new(1, 0).unwrap_err();

        assert_eq!(err.kind(), crate::AvErrorKind::InvalidArgument);
    }

    #[test]
    fn rational_compares_without_float_rounding() {
        assert!(Rational::new(1, 2).unwrap() < Rational::new(2, 3).unwrap());
        assert_eq!(
            Rational::new(1001, 30000).unwrap(),
            Rational::new(2002, 60000).unwrap()
        );
    }

    #[test]
    fn rational_av_cmp_matches_ffmpeg_sentinels() {
        assert_eq!(
            Rational::new(1, 2)
                .unwrap()
                .av_cmp(Rational::new(2, 3).unwrap()),
            Some(Ordering::Less)
        );
        assert_eq!(
            Rational::from_raw(1, -2).av_cmp(Rational::new(1, 3).unwrap()),
            Some(Ordering::Less)
        );
        assert_eq!(
            Rational::from_raw(1, 0).av_cmp(Rational::from_raw(2, 0)),
            Some(Ordering::Equal)
        );
        assert_eq!(
            Rational::from_raw(1, 0).av_cmp(Rational::from_raw(-1, 0)),
            Some(Ordering::Greater)
        );
        assert_eq!(
            Rational::from_raw(1, 0).av_cmp(Rational::new(24, 1).unwrap()),
            Some(Ordering::Greater)
        );
        assert_eq!(
            Rational::from_raw(0, 0).av_cmp(Rational::new(1, 1).unwrap()),
            None
        );
    }

    #[test]
    fn rational_nearer_to_reports_closer_candidate_and_ties() {
        let reference = Rational::new(25, 1).unwrap();

        assert_eq!(
            reference
                .nearer_to(Rational::new(24, 1).unwrap(), Rational::new(30, 1).unwrap())
                .unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            reference
                .nearer_to(Rational::new(20, 1).unwrap(), Rational::new(26, 1).unwrap())
                .unwrap(),
            Ordering::Less
        );
        assert_eq!(
            Rational::new(27, 1)
                .unwrap()
                .nearer_to(Rational::new(24, 1).unwrap(), Rational::new(30, 1).unwrap())
                .unwrap(),
            Ordering::Equal
        );
        assert_eq!(
            Rational::new(1001, 30000)
                .unwrap()
                .nearer_to(
                    Rational::new(1, 30).unwrap(),
                    Rational::new(1001, 30000).unwrap()
                )
                .unwrap(),
            Ordering::Less
        );
    }

    #[test]
    fn rational_find_nearest_index_keeps_first_tie() {
        let candidates = [
            Rational::new(24, 1).unwrap(),
            Rational::new(30, 1).unwrap(),
            Rational::new(25, 1).unwrap(),
        ];

        assert_eq!(
            Rational::new(26, 1)
                .unwrap()
                .find_nearest_index(&candidates)
                .unwrap(),
            Some(2)
        );
        assert_eq!(
            Rational::new(27, 1)
                .unwrap()
                .find_nearest_index(&candidates[..2])
                .unwrap(),
            Some(0)
        );
        assert_eq!(
            Rational::new(27, 1)
                .unwrap()
                .find_nearest_index(&[])
                .unwrap(),
            None
        );
    }

    #[test]
    fn rational_nearest_helpers_reject_zero_denominators() {
        assert_eq!(
            Rational::from_raw(1, 0)
                .nearer_to(Rational::new(24, 1).unwrap(), Rational::new(30, 1).unwrap())
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(
            Rational::new(25, 1)
                .unwrap()
                .find_nearest_index(&[Rational::new(24, 1).unwrap(), Rational::from_raw(0, 0)])
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidArgument
        );
    }

    #[test]
    fn rational_gcd_with_limit_matches_ffmpeg_shape() {
        assert_eq!(
            Rational::new(1, 30)
                .unwrap()
                .gcd_with_limit(Rational::new(1, 60).unwrap(), 100, Rational::ONE)
                .unwrap(),
            Rational::from_raw(1, 60)
        );
        assert_eq!(
            Rational::new(2, 3)
                .unwrap()
                .gcd_with_limit(Rational::new(4, 9).unwrap(), 10, Rational::ONE)
                .unwrap(),
            Rational::from_raw(2, 9)
        );
        assert_eq!(
            Rational::from_raw(2, 4)
                .gcd_with_limit(Rational::from_raw(4, 6), 100, Rational::ONE)
                .unwrap(),
            Rational::from_raw(2, 12)
        );
        assert_eq!(
            Rational::from_raw(0, 5)
                .gcd_with_limit(Rational::from_raw(10, 15), 100, Rational::ONE)
                .unwrap(),
            Rational::from_raw(10, 15)
        );
    }

    #[test]
    fn rational_gcd_with_limit_uses_strict_max_den_and_default() {
        let default = Rational::from_raw(30000, 1001);

        assert_eq!(
            Rational::new(1, 30)
                .unwrap()
                .gcd_with_limit(Rational::new(1, 60).unwrap(), 60, default)
                .unwrap(),
            default
        );
        assert_eq!(
            Rational::new(1, 30)
                .unwrap()
                .gcd_with_limit(Rational::new(1, 60).unwrap(), 59, default)
                .unwrap(),
            default
        );
    }

    #[test]
    fn rational_gcd_with_limit_rejects_invalid_inputs() {
        assert_eq!(
            Rational::new(1, 30)
                .unwrap()
                .gcd_with_limit(Rational::new(1, 60).unwrap(), 0, Rational::ONE)
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(
            Rational::from_raw(1, 0)
                .gcd_with_limit(Rational::new(1, 60).unwrap(), 100, Rational::ONE)
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(
            Rational::from_raw(1, -30)
                .gcd_with_limit(Rational::new(1, 60).unwrap(), 100, Rational::ONE)
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidArgument
        );
    }

    #[test]
    fn rational_arithmetic_reduces_results() {
        let half = Rational::new(1, 2).unwrap();
        let third = Rational::new(1, 3).unwrap();

        assert_eq!(
            half.checked_add(third).unwrap(),
            Rational::new(5, 6).unwrap()
        );
        assert_eq!(
            half.checked_sub(third).unwrap(),
            Rational::new(1, 6).unwrap()
        );
        assert_eq!(
            third.checked_sub(half).unwrap(),
            Rational::new(-1, 6).unwrap()
        );
        assert_eq!(
            Rational::new(2, 3)
                .unwrap()
                .checked_mul(Rational::new(9, 4).unwrap())
                .unwrap(),
            Rational::new(3, 2).unwrap()
        );
        assert_eq!(
            Rational::new(3, 2)
                .unwrap()
                .checked_div(Rational::new(9, 4).unwrap())
                .unwrap(),
            Rational::new(2, 3).unwrap()
        );
        assert_eq!(
            Rational::new(-7, 11).unwrap().checked_neg().unwrap(),
            Rational::new(7, 11).unwrap()
        );
        assert_eq!(
            Rational::from_raw(i32::MIN, i32::MIN)
                .checked_add(Rational::from_raw(i32::MIN, i32::MIN))
                .unwrap(),
            Rational::new(2, 1).unwrap()
        );
    }

    #[test]
    fn rational_arithmetic_rejects_invalid_or_out_of_range_results() {
        assert_eq!(
            Rational::from_raw(1, 0)
                .checked_add(Rational::ONE)
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(
            Rational::from_raw(i32::MAX, 1)
                .checked_add(Rational::ONE)
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidArgument
        );
        assert_eq!(
            Rational::from_raw(i32::MIN, 1)
                .checked_neg()
                .unwrap_err()
                .kind(),
            crate::AvErrorKind::InvalidArgument
        );
    }

    #[test]
    fn rational_reduce_i64_reports_exact_limited_results() {
        assert_eq!(
            Rational::reduce_i64(30000, -1001, i32::MAX).unwrap(),
            (Rational::new(-30000, 1001).unwrap(), true)
        );
        assert_eq!(
            Rational::reduce_i64(0, 4000, 100).unwrap(),
            (Rational::from_raw(0, 1), true)
        );
        assert_eq!(
            Rational::reduce_i64(42, 0, 100).unwrap(),
            (Rational::from_raw(1, 0), true)
        );
        assert_eq!(
            Rational::reduce_i64(0, 0, 100).unwrap(),
            (Rational::from_raw(0, 0), true)
        );
    }

    #[test]
    fn rational_reduce_i64_approximates_when_limited() {
        assert_eq!(
            Rational::reduce_i64(1001, 30000, 100).unwrap(),
            (Rational::new(1, 30).unwrap(), false)
        );
        assert_eq!(
            Rational::reduce_i64(2, 3, 1).unwrap(),
            (Rational::new(1, 1).unwrap(), false)
        );
        assert_eq!(
            Rational::reduce_i64(-2, 3, 1).unwrap(),
            (Rational::new(-1, 1).unwrap(), false)
        );
    }

    #[test]
    fn rational_reduce_i64_rejects_invalid_limits() {
        assert_eq!(
            Rational::reduce_i64(1, 2, 0).unwrap_err().kind(),
            crate::AvErrorKind::InvalidArgument
        );
    }

    #[test]
    fn rational_from_f64_limited_handles_finite_and_special_values() {
        assert_eq!(
            Rational::from_f64_limited(0.5, 100).unwrap(),
            Rational::new(1, 2).unwrap()
        );
        assert_eq!(
            Rational::from_f64_limited(-0.25, 100).unwrap(),
            Rational::new(-1, 4).unwrap()
        );
        assert_eq!(
            Rational::from_f64_limited(29.97, 3000).unwrap(),
            Rational::new(2997, 100).unwrap()
        );
        assert_eq!(
            Rational::from_f64_limited(29.97, 1001).unwrap(),
            Rational::new(989, 33).unwrap()
        );
        assert_eq!(
            Rational::from_f64_limited(f64::NAN, 100).unwrap(),
            Rational::from_raw(0, 0)
        );
        assert_eq!(
            Rational::from_f64_limited(f64::INFINITY, 100).unwrap(),
            Rational::from_raw(1, 0)
        );
        assert_eq!(
            Rational::from_f64_limited(f64::NEG_INFINITY, 100).unwrap(),
            Rational::from_raw(-1, 0)
        );
    }

    #[test]
    fn rational_from_f64_limited_rejects_invalid_limits() {
        assert_eq!(
            Rational::from_f64_limited(0.5, 0).unwrap_err().kind(),
            crate::AvErrorKind::InvalidArgument
        );
    }
}
