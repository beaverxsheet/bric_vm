//! Bens Reduced Instruction set Computer Virtual Machine
//!
//! A VM for a [`NAND-Game`] inspired architecture.
//!
//! [`NAND-Game`]: https://nandgame.com

pub use util::BError;

pub mod assembler;
pub mod disassembler;
pub mod util;

/// Debugging BRICs
pub mod debugger;

pub mod mmio;

/// Routines for simulating a BRIC
pub mod vm;
