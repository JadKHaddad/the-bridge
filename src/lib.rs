#![no_std]

pub trait Captures<U> {}

impl<T: ?Sized, U> Captures<U> for T {}

use bincode::{Decode, Encode};
use core::fmt::Debug;

#[derive(Debug, Clone, Encode, Decode, PartialEq)]
pub enum Message {
    A(i32),
    B(u32),
    C(
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
        i32,
    ),
}

pub mod decode;
pub mod encode;

#[cfg(feature = "futures")]
pub mod futures;

#[cfg(feature = "embedded-io")]
pub mod embedded_io;

// TODO: tokio-util codec
#[cfg(feature = "tokio")]
pub mod tokio;

#[cfg(test)]
mod tests;
