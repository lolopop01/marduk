//! GPU device + surface management.
//!
//! This module is responsible for:
//! - creating the wgpu Instance/Adapter/Device/Queue
//! - creating & configuring the Surface (swapchain)
//! - acquiring frames and providing encoders/views for rendering

mod gpu;

pub use gpu::{Gpu, GpuFrame, GpuInit};