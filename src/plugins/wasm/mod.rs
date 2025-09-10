//! WASM module for plugin runtime support.

pub mod runtime;

pub use runtime::{WasmRuntime, PluginInfo, HostFunctions};