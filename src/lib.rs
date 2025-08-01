#![deny(missing_docs)]
//! # aerosol
//! Simple but powerful dependency injection for Rust.
//!
//! This crate provides the `Aero` type, which stores dependencies (called resources) keyed by their
//! type. Resources can be constructed eagerly at application startup, or on-demand when they are
//! first needed. Resources can access and/or initialize other resources on creation.
//!
//! The crate will detect dependency cycles (if constructing resource A requires resource B which
//! itself requires resource A) and will panic rather than stack overflow in that case.
//!
//! The `Aero` type has an optional type parameter to make certain resources *required*. When
//! a resource is required it can be accessed infallibly. The `Aero![...]` macro exists to
//! easily name an `Aero` with a specific set of required resources.
//!
//! Cloning or type casting an `Aero` type is cheap (equivalent to cloning an `Arc`).
//!
//! ## Optional features
//!
//! ### `async`
//!
//! Allows resources to be constructed asynchrously, and provides a corresponding
//! `AsyncConstructibleResource` trait.
//!
//! ### `axum`
//!
//! Provides integrations with the `axum` web framework. See the `axum` module
//! for more information.
//!
//! ### `aide`
//!
//! Provides integrations with the `aide` library. Implements the `OperationInput` trait
//! for `Dep<T>` and `Obtain<T>` types, allowing them to be used in `aide`-generated OpenAPI
//! documentation.
//!
//! ## Example usage
//!
//! ```rust
//! use std::{sync::Arc, any::Any};
//!
//! # struct PostmarkClient;
//! # #[derive(Clone)]
//! # struct ConnectionPool;
//! # #[derive(Clone)]
//! # struct MessageQueue;
//! # #[derive(Clone)]
//! # struct MagicNumber(i32);
//! # trait EmailSender: Send + Sync { fn send(&self) {} }
//! # impl EmailSender for PostmarkClient {}
//! # impl PostmarkClient { fn new() -> anyhow::Result<Self> { Ok(Self) }}
//! use aerosol::{Aero, Constructible};
//!
//! // Here, we can list all the things we want to guarantee are in
//! // our app state. This is entirely optional, we could also just
//! // use the `Aero` type with default arguments and check that
//! // resources are present at runtime.
//! type AppState = Aero![
//!     Arc<PostmarkClient>,
//!     Arc<dyn EmailSender>,
//!     ConnectionPool,
//!     MessageQueue,
//!     MagicNumber,
//! ];
//!
//! fn main() {
//!     let app_state: AppState = Aero::new()
//!         // Directly add a resource which doesn't implement `Constructible`.
//!         .with(MagicNumber(42))
//!         // Construct an `Arc<PostmarkClient>` resource in the AppState
//!         .with_constructed::<Arc<PostmarkClient>>()
//!         // Check that an implementation of `EmailSender` was added as a result
//!         .assert::<Arc<dyn EmailSender>>()
//!         // Automatically construct anything else necessary for our AppState
//!         // (in this case, `ConnectionPool` and `MessageQueue`)
//!         .construct_remaining();
//!
//!     // Add an extra resource
//!     app_state.insert("Hello, world");
//!
//!     run(app_state);
//! }
//!
//! fn run(app_state: AppState) {
//!     // The `get()` method is infallible because the `Arc<dyn EmailSender>` was
//!     // explicitly listed when defining our `AppState`.
//!     let email_sender: Arc<dyn EmailSender> = app_state.get();
//!     email_sender.send(/* email */);
//!
//!     // We have to use `try_get()` here because a `&str` is not guaranteed to
//!     // exist on our `AppState`.
//!     let hello_message: &str = app_state.try_get().unwrap();
//!     println!("{hello_message}");
//!
//!     // ... more application logic
//! }
//!
//! // The `Constructible` trait can be implemented to allow resources to be automatically
//! // constructed.
//! impl Constructible for PostmarkClient {
//!     type Error = anyhow::Error;
//!
//!     fn construct(aero: &Aero) -> Result<Self, Self::Error> {
//!         PostmarkClient::new(/* initialize using environment variables */)
//!     }
//!
//!     fn after_construction(this: &(dyn Any + Send + Sync), aero: &Aero) -> Result<(), Self::Error> {
//!         // We can use this to automatically populate extra resources on the context.
//!         // For example, in this case we can make it so that if an `Arc<PostmarkClient>` gets
//!         // constructed, we also provide `Arc<dyn EmailSender>`.
//!         if let Some(arc) = this.downcast_ref::<Arc<Self>>() {
//!             aero.insert(arc.clone() as Arc<dyn EmailSender>)
//!         }
//!         Ok(())
//!     }
//! }
//!
//! impl Constructible for ConnectionPool {
//!     type Error = anyhow::Error;
//!     fn construct(aero: &Aero) -> Result<Self, Self::Error> {
//!         // ...
//! #       Ok(ConnectionPool)
//!     }
//! }
//!
//! impl Constructible for MessageQueue {
//!     type Error = anyhow::Error;
//!     fn construct(aero: &Aero) -> Result<Self, Self::Error> {
//!         // ...
//! #       Ok(MessageQueue)
//!     }
//! }
//! ```
//!
//! ## Implementation details
//!
//! The `Aero` type manages shared ownership of a map from resource types to "slots".
//! For a given resource type, the corresponding "slot" can be in one of three state:
//! 1) Absent.
//!    No instance of this resource is present in the map.
//! 2) Present.
//!    An instance of this resource exists in the map and can be accessed immediately.
//! 3) Under construction.
//!    An instance of this resource is currently under construction, and may be accessed
//!    once construction has finished.
//!    The slot maintains a list of threads or tasks waiting for this resource to be
//!    constructed, and will wake them when the resource becomes available.
//!
//! Resources can be constructed synchronously, or (when the feature is enabled) asynchronously.
//!
//! If a resource is accessed whilst under construction, the caller will wait for construction
//! to complete. The caller determines whether the wait occurs synchronously or asynchronously,
//! depending on whether `obtain()` or `obtain_async()` is used.
//!
//! It is possible (and allowed) for a thread to synchronously wait on a resource being constructed
//! asynchronously in a task, or for a task to asynchronously wait on a resource being synchronously
//! constructed on a thread.
//!

pub use frunk;

#[cfg(feature = "async")]
mod async_;
#[cfg(feature = "async")]
mod async_constructible;
#[cfg(feature = "axum")]
pub mod axum;
mod macros;
mod resource;
mod slot;
mod state;
mod sync;
mod sync_constructible;

pub use resource::{Resource, ResourceList};
pub use state::Aero;

pub use sync_constructible::{
    Constructible, ConstructibleResource, ConstructibleResourceList, IndirectlyConstructible,
};

#[cfg(feature = "async")]
pub use async_constructible::{
    AsyncConstructible, AsyncConstructibleResource, AsyncConstructibleResourceList,
    IndirectlyAsyncConstructible,
};
