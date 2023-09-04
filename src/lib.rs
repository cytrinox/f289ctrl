//!
//! This library provides communication with a Fluke 287/289 digital multimeter.
//!
//! <br>
//!
//! # Details
//!
//! - You need a Fluke IR cable attached to your DMM.
//!

pub mod device;
pub mod measurement;
pub mod proto;
pub mod rawmea;

pub use device::Device;
pub use proto::Result;

#[cfg(unix)]
pub const DEFAULT_TTY: &str = "/dev/ttyUSB0";
#[cfg(windows)]
pub const DEFAULT_TTY: &str = "COM1";

/// Default Baudrate for Fluke 287 and 289.
pub const DEFAULT_BAUDRATE: u32 = 115200;
