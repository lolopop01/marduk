//! GPU device + surface management.
//!
//! Responsibility:
//! - wgpu instance/adapter/device/queue creation
//! - surface creation and configuration
//! - frame acquisition and command submission

mod context;
mod error;
mod frame;
mod init;
mod surface;

pub use context::Gpu;
pub use error::SurfaceErrorAction;
pub use frame::GpuFrame;
pub use init::GpuInit;