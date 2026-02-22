//! Logging utilities.
//!
//! This module centralizes logger initialization and common diagnostics.
//! It is intentionally small and avoids imposing a specific logging backend
//! beyond the standard `log` facade.

mod init;

pub use init::{init_logging, LoggingConfig};