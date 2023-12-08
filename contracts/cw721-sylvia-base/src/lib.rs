#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

pub mod base;
pub mod contract;
mod error;
pub mod responses;
pub mod state;

#[cfg(test)]
mod multitest;

pub use crate::error::ContractError;
