//! # aerosol
//! Simple dependency injection for Rust
//! 
//! The two main exports of this crate are the `define_context`
//! and `define_interface` macros.
//! 
//! Contexts are containers for multiple dependencies, allowing
//! them to be passed around as one with relative ease. Interfaces
//! are specialized traits which place constraints on contexts,
//! indicating exactly what dependencies a context must provide.
//! 
//! Contexts are typically created at the top level of an application,
//! as they specify exactly what concrete versions of all dependencies
//! are going to be used. A single context is created with a precise
//! set of depenencies, and is then threaded through the rest of the
//! application as a generic parameter.
//! 
//! Interfaces are used at every level of an application, as they
//! allow each piece of code to independently specify what dependencies
//! are required. Interfaces can "inherit" the dependencies of other
//! interfaces, with the idea being that this inheritance will form
//! a tree, such that there will be some "root interface" which contains
//! the union of all dependencies required by the whole application.
//! 
//! This pattern allows dependencies to be added or removed from any
//! part of the application without having to modify the code at every
//! level, to thread or un-thread the new or old dependencies through.
//! 
//! ## Example
//! 
//! ```
//! #![recursion_limit="128"]
//! use std::sync::Arc;
//! use std::fmt::Debug;
//! use failure;
//! 
//! // We will depend on some kind of logger
//! trait Logger: Debug {
//!     fn log(&self, msg: &str);
//! }
//! 
//! // We have a specific implementation of a stdout logger
//! #[derive(Debug)]
//! struct StdoutLogger;
//! 
//! impl Logger for StdoutLogger {
//!     fn log(&self, msg: &str) {
//!         println!("{}", msg);
//!     }
//! }
//! 
//! struct StdoutLoggerFactory;
//! impl aerosol::Factory for StdoutLoggerFactory {
//!     type Object = Arc<Logger>;
//!     fn build(_: ()) -> Result<Arc<Logger>, failure::Error> {
//!         Ok(Arc::new(StdoutLogger))
//!     }
//! }
//! 
//! // Part of our application does some work
//! aerosol::define_interface!(
//!     WorkerInterface {
//!         fn logger(&self) -> Arc<Logger>;
//!     }
//! );
//! 
//! fn do_work<I: WorkerInterface>(iface: I) {
//!     iface.logger().log("Doing some work!");
//! }
//! 
//! // Our application does multiple pieces of work
//! aerosol::define_interface!(
//!     AppInterface: WorkerInterface + Clone {}
//! );
//! 
//! fn run_app<I: AppInterface>(iface: I, num_work_items: usize) {
//!     for _ in 0..num_work_items {
//!         do_work(iface.clone());
//!     }
//! }
//! 
//! // At the very top level, we specify the implementations
//! // of our dependencies.
//! aerosol::define_context!(
//!     AppContext {
//!         logger: Arc<Logger> [StdoutLoggerFactory],
//!     }
//! );
//! 
//! fn main() {
//!     let context = AppContext::new().unwrap();
//! 
//!     run_app(context, 4);
//! }
//! ```
//! 
//! See the individual macro documentation for more details.

#[doc(hidden)]
pub extern crate tt_call;
#[doc(hidden)]
pub extern crate failure;

mod join;
mod parse;
mod interface;
mod context;


/// The building block for this crate. Automatically implemented
/// for contexts providing a dependency of type `T`.
/// 
/// Super-trait of all interfaces requiring a dependency of type
/// `T`.
pub trait Provide<T> {
    fn provide(&self) -> T;
}

/// Implement this trait to provide a convenient syntax for
/// constructing implementations of dependencies.
pub trait Factory<Args=()> {
    type Object;
    fn build(args: Args) -> Result<Self::Object, failure::Error>;
}

/// Allows cloning a context whilst replacing one dependency
/// with a different implementation. Must be explicitly listed
/// as a super-trait of an interface to use.
pub trait ProvideWith<T>: Provide<T> + Sized {
    fn provide_with<E, F: FnOnce(T) -> Result<T, E>>(&self, f: F) -> Result<Self, E>;
}
