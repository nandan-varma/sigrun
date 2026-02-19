//! Common userspace types and utilities

#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]

pub mod buffer;
pub mod error;
pub mod handle;
pub mod ipc;

pub use error::Error;
pub use handle::*;
