#![cfg_attr(not(feature = "std"), no_std)]

pub mod codec;

#[cfg(feature = "cody-c")]
mod cody_c;

#[cfg(feature = "tokio")]
mod tokio;

#[cfg(test)]
pub mod test;
