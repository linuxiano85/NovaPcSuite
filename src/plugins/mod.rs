//! Plugin system for NovaPcSuite extensibility.
//! 
//! This module provides a plugin architecture with WASM runtime support
//! and an event system for plugin communication.

pub mod events;

#[cfg(feature = "wasm-plugins")]
pub mod wasm;

// Re-export main types
pub use events::{PlatformEvent, EventBus};

#[cfg(feature = "wasm-plugins")]
pub use wasm::runtime::WasmRuntime;