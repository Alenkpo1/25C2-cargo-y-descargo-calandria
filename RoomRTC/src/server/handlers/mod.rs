//! Módulo de handlers para mensajes del protocolo de señalización.

pub mod auth;
pub mod presence;
pub mod signaling;

mod context;
pub use context::{dispatch, HandlerResult};
