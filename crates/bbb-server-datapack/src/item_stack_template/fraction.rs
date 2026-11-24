//! The commons-lang3 3.19.0 `Fraction` arithmetic `BundleContents.weight`
//! (`BundleContents.java:103-105`) is computed in. Only whether it throws is
//! observable through `ItemStack.validateComponents`, so each
//! `ArithmeticException` its `int` numerator and denominator raise is one
//! `None`.

/// `org.apache.commons.lang3.math.Fraction`, which stores a raw numerator and
/// denominator and reduces only where its own operations do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Fraction {
    numerator: i32,
    denominator: i32,
}

impl Fraction {
    pub(super) const ZERO: Self = Self::of(0, 1);
    pub(super) const ONE: Self = Self::of(1, 1);

    /// The private `Fraction(int, int)` constructor: neither sign-normalized
    /// nor reduced.
    pub(super) const fn of(numerator: i32, denominator: i32) -> Self {
        Self {
            numerator,
            denominator,
        }
    }

    /// `Fraction.getFraction(int, int)`: the sign moves to the numerator and
    /// nothing else changes.
    pub(super) fn get(numerator: i32, denominator: i32) -> Option<Self> {
        if denominator == 0 {
            return None;
        }
        if denominator < 0 {
            if numerator == i32::MIN || denominator == i32::MIN {
                return None;
            }
            return Some(Self::of(-numerator, -denominator));
        }
        Some(Self::of(numerator, denominator))
    }

    /// `Fraction.getReducedFraction`.
    fn reduced(mut numerator: i32, mut denominator: i32) -> Option<Self> {
        if denominator == 0 {
            return None;
        }
        if numerator == 0 {
            return Some(Self::ZERO);
        }
        if denominator == i32::MIN && numerator % 2 == 0 {
            numerator /= 2;
            denominator /= 2;
        }
        if denominator < 0 {
            if numerator == i32::MIN || denominator == i32::MIN {
                return None;
            }
            numerator = -numerator;
            denominator = -denominator;
        }
        let divisor = greatest_common_divisor(numerator, denominator)?;
        Some(Self::of(numerator / divisor, denominator / divisor))
    }

    /// `Fraction.add` through `addSub`: coprime denominators multiply out and
    /// bound every cross product, and a shared divisor takes the exact path
    /// that bounds only the partly reduced result.
    pub(super) fn add(self, other: Self) -> Option<Self> {
        if self.numerator == 0 {
            return Some(other);
        }
        if other.numerator == 0 {
            return Some(self);
        }
        let shared = greatest_common_divisor(self.denominator, other.denominator)?;
        if shared == 1 {
            let left = mul_and_check(self.numerator, other.denominator)?;
            let right = mul_and_check(other.numerator, self.denominator)?;
            return Some(Self::of(
                add_and_check(left, right)?,
                mul_pos_and_check(self.denominator, other.denominator)?,
            ));
        }
        let left = i128::from(self.numerator) * i128::from(other.denominator / shared);
        let right = i128::from(other.numerator) * i128::from(self.denominator / shared);
        let sum = left + right;
        let remainder = i32::try_from(sum.rem_euclid(i128::from(shared))).ok()?;
        let divisor = if remainder == 0 {
            shared
        } else {
            greatest_common_divisor(remainder, shared)?
        };
        Some(Self::of(
            i32::try_from(sum / i128::from(divisor)).ok()?,
            mul_pos_and_check(self.denominator / shared, other.denominator / divisor)?,
        ))
    }

    /// `Fraction.multiplyBy`, which cross-reduces before it multiplies.
    pub(super) fn multiply_by(self, other: Self) -> Option<Self> {
        if self.numerator == 0 || other.numerator == 0 {
            return Some(Self::ZERO);
        }
        let left = greatest_common_divisor(self.numerator, other.denominator)?;
        let right = greatest_common_divisor(other.numerator, self.denominator)?;
        Self::reduced(
            mul_and_check(self.numerator / left, other.numerator / right)?,
            mul_pos_and_check(self.denominator / right, other.denominator / left)?,
        )
    }
}

/// `Fraction.greatestCommonDivisor`: the positive divisor, and the `2^31` it
/// refuses to return rather than the two inputs that would produce it.
fn greatest_common_divisor(left: i32, right: i32) -> Option<i32> {
    if left == 0 || right == 0 {
        return (left != i32::MIN && right != i32::MIN).then(|| left.abs() + right.abs());
    }
    if left == i32::MIN && right == i32::MIN {
        return None;
    }
    let (mut divisor, mut remainder) = (left.unsigned_abs(), right.unsigned_abs());
    while remainder != 0 {
        (divisor, remainder) = (remainder, divisor % remainder);
    }
    i32::try_from(divisor).ok()
}

/// `Fraction.mulAndCheck` and `Fraction.addAndCheck`, which bound their whole
/// result to `int`.
fn mul_and_check(left: i32, right: i32) -> Option<i32> {
    i32::try_from(i64::from(left) * i64::from(right)).ok()
}

fn add_and_check(left: i32, right: i32) -> Option<i32> {
    i32::try_from(i64::from(left) + i64::from(right)).ok()
}

/// `Fraction.mulPosAndCheck`, which bounds only the upper end and narrows the
/// rest.
fn mul_pos_and_check(left: i32, right: i32) -> Option<i32> {
    let product = i64::from(left) * i64::from(right);
    (product <= i64::from(i32::MAX)).then_some(product as i32)
}
