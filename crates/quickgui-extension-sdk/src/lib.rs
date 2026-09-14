//! Native component declarations and lifecycle helpers. This crate has no windowing, layout,
//! text shaping, GPU, or application runtime dependency.
pub mod abi;
pub mod builder;
pub mod runtime;
pub mod schema;

pub use runtime::{Component, ComponentFactory, ComponentRuntime, WakeHandle};
pub use schema::*;
