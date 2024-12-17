use std::fmt::{Debug, Display};
use std::iter::Sum;
use std::ops::{Add, Div, Mul, Neg, Sub};

pub trait Numeric:
    Add<Output = Self>
    + Sum
    + Mul<Output = Self>
    + Div<Output = Self>
    + Invertible
    + Sub<Output = Self>
    + Default
    + Copy
    + Debug
    + Display
    + Default
    + PartialEq
    + Eq
    + PartialOrd
    + Ord
{
}
impl<T> Numeric for T where
    T: Add<Output = T>
        + Sum
        + Sub<Output = T>
        + Mul<Output = Self>
        + Div<Output = Self>
        + Invertible
        + Default
        + Copy
        + Debug
        + Display
        + Default
        + PartialEq
        + Eq
        + PartialOrd
        + Ord
{
}

pub trait NumericNeg: Numeric + Neg<Output = Self> {}
impl<T> NumericNeg for T where T: Numeric + Neg<Output = Self> {}

pub trait Invertible {
    fn invert(self) -> Self;
}

impl Invertible for i32 {
    fn invert(self) -> Self {
        1 / self
    }
}

impl Invertible for i64 {
    fn invert(self) -> Self {
        1 / self
    }
}

impl Invertible for usize {
    fn invert(self) -> Self {
        1 / self
    }
}

impl Invertible for f32 {
    fn invert(self) -> Self {
        1.0 / self
    }
}

impl Invertible for f64 {
    fn invert(self) -> Self {
        1.0 / self
    }
}
