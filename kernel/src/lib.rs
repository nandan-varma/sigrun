//! SIGRUN Kernel Library crate root.
//!
//! This file exists so that the package can be treated as a library target
//! (e.g. for `cargo doc` or future unit-test infrastructure).
//! The actual kernel binary entry point and all module declarations live in
//! main.rs (the binary target root).
//!
//! DO NOT add `mod` declarations here — that would cause the boot assembly
//! (with `_start`) to be compiled twice (once for lib, once for bin) leading
//! to duplicate-symbol link errors.

#![no_std]
#![allow(unused)]
