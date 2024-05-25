#![cfg_attr(not(feature = "std"), no_std)]

pub mod decode;
pub mod encode;

#[cfg(feature = "futures")]
pub mod captures;

#[cfg(feature = "futures-io")]
pub mod futures_io;

#[cfg(feature = "embedded-io")]
pub mod embedded_io;

#[cfg(feature = "tokio")]
pub mod tokio;

#[cfg(test)]
mod tests;

#[cfg(feature = "demo")]
pub mod demo;
