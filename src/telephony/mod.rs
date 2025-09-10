//! Telephony abstraction for companion app integration.
//! 
//! This module provides async traits and abstractions for telephony services,
//! enabling integration with companion mobile apps for notifications and remote control.

pub mod provider;

// Re-export main types
pub use provider::{TelephonyProvider, TelephonyEvent, NotificationLevel, CallDirection};