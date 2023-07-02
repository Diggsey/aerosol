#![deny(missing_docs)]
//! # aerosol
//! Simple dependency injection for Rust
//!
//! Optional features:
//!
//! ## `async`
//!
//! Allows resources to be constructed asynchrously, and provides a corresponding
//! `AsyncConstructibleResource` trait.
//!
//! ## `axum`
//!
//! Provides integrations with the `axum` web framework. See the `axum` module
//! for more information.

#[cfg(feature = "async")]
mod async_;
#[cfg(feature = "async")]
mod async_constructible;
#[cfg(feature = "axum")]
pub mod axum;
mod resource;
mod slot;
mod state;
mod sync;
mod sync_constructible;

pub use resource::Resource;
pub use state::Aerosol;

pub use sync_constructible::ConstructibleResource;

#[cfg(feature = "async")]
pub use async_constructible::AsyncConstructibleResource;
