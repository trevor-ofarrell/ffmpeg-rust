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

    pub fn reciprocal(self) -> AvResult<Self> {
        if self.num == 0 {
            return Err(AvError::invalid_argument("cannot invert zero rational"));
        }

        Self::new(self.den, self.num)
    }

    pub fn checked_mul(self, other: Self) -> AvResult<Self> {
        let num = i64::from(self.num) * i64::from(other.num);
        let den = i64::from(self.den) * i64::from(other.den);
        rational_from_i64(num, den)
    }

    pub fn checked_div(self, other: Self) -> AvResult<Self> {
        self.checked_mul(other.reciprocal()?)
    }
}

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let left = i64::from(self.num) * i64::from(other.den);
        let right = i64::from(other.num) * i64::from(self.den);
        left.partial_cmp(&right)
    }
}

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.num, self.den)
    }
}

fn rational_from_i64(num: i64, den: i64) -> AvResult<Rational> {
    if den == 0 {
        return Err(AvError::invalid_argument(
            "rational denominator must not be zero",
        ));
    }

    let divisor = gcd_i64(num, den);
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
}
