//! Common userspace types and utilities

#![no_std]

pub mod error;

pub mod ipc {
    use crate::error::Error;
    
    pub type Result<T> = core::result::Result<T, Error>;
}
