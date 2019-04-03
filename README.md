# aerosol

Simple dependency injection for Rust

[Documentation](https://docs.rs/aerosol/)

The two main exports of this crate are the `define_context!` and `define_interface!` macros.

Contexts are containers for multiple dependencies, allowing them to be passed around as one with relative ease. Interfaces are specialized traits which place constraints on contexts, indicating exactly what dependencies a context must provide.

Contexts are typically created at the top level of an application, as they specify exactly what concrete versions of all dependencies are going to be used. A single context is created with a precise set of depenencies, and is then threaded through the rest of the application as a generic parameter.

Interfaces are used at every level of an application, as they allow each piece of code to independently specify what dependencies are required. Interfaces can "inherit" the dependencies of other interfaces, with the idea being that this inheritance will form a tree, such that there will be some "root interface" which contains the union of all dependencies required by the whole application.

This pattern allows dependencies to be added or removed from any part of the application without having to modify the code at every level, to thread or un-thread the new or old dependencies through.
