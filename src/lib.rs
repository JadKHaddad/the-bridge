#![no_std]

pub mod decode;
pub mod encode;

#[cfg(feature = "futures")]
pub mod captures;

#[cfg(feature = "futures")]
pub mod futures;

#[cfg(feature = "embedded-io")]
pub mod embedded_io;

#[cfg(feature = "tokio")]
pub mod tokio;

#[cfg(test)]
mod tests;

#[cfg(feature = "demo")]
pub mod demo;
