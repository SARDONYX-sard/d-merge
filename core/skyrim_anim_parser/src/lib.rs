//! # Animation Data Modules
//!
//! This crate provides modules for handling animation-related data files:
//!
//! - `adsf`: Represents `animationdatasinglefile.txt`
//! - `asdsf`: Represents `animationsetdatasinglefile.txt`
//!
//! Each module is organized into three variants:
//!
//! - **alt**:
//!   Contains data structures transformed for easier application of Nemesis patches.
//!   These are optimized for runtime performance and serve as templates for patching.
//!
//! - **normal**:
//!   Directly parsed data structures from vanilla Skyrim files, maintaining original layout.
//!
//! - **patch**:
//!   Code for analyzing diffs from Nemesis patches, enabling interpretation and comparison.
//!
//! Each variant contains submodules:
//!
//! - `ser`: Serialization logic
//! - `de`: Deserialization logic
pub mod adsf;
pub mod asdsf;
mod lines;
