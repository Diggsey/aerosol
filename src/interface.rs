#[doc(hidden)]
#[macro_export]
macro_rules! generate_trait_def {
    {
        $caller:tt
        name = [{ $name:ident }]
        bounds = [{ $($bounds:tt)+ }]
        getters = [{ $(
            { $getter:ident $t:ty }
        )* }]
    } => {
        $crate::tt_call::tt_return! {
            $caller
            trait_def = [{
                pub trait $name: $($bounds)+ {
                    $(fn $getter(&self) -> $t;)*
                }
            }]
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! generate_trait_impl {
    {
        $caller:tt
        name = [{ $name:ident }]
        bounds = [{ $($bounds:tt)+ }]
        getters = [{ $(
            { $getter:ident $t:ty }
        )* }]
    } => {
        $crate::tt_call::tt_return! {
            $caller
            trait_impl = [{
                impl<T: $($bounds)+> $name for T {
                    $(fn $getter(&self) -> $t {
                        $crate::Provide::<$t>::provide(self)
                    })*
                }
            }]
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_define_interface {
    {
        $caller:tt
        input = [{ $($input:tt)* }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ $crate::parse_trait_def }]
            input = [{ $($input)* }]
            ~~> $crate::private_define_interface! {
                $caller
            }
        }
    };
    {
        $caller:tt
        name = [{ $name:ident }]
        body = [{ $(
            fn $getter:ident(&self) -> $t:ty;
        )* }]
        $(bound = [{ $($bound:tt)* }])*
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ $crate::join }]
            sep = [{ + }]
            $(item = [{ $($bound)* }])*
            $(item = [{ $crate::Provide<$t> }])*
            ~~> $crate::private_define_interface! {
                $caller
                name = [{ $name }]
                getters = [{ $(
                    { $getter $t }
                )* }]
            }
        }
    };
    {
        $caller:tt
        name = [{ $name:ident }]
        getters = [{ $($getters:tt)* }]
        joined = [{ $($joined:tt)* }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ $crate::generate_trait_def }]
            name = [{ $name }]
            bounds = [{ $($joined)* }]
            getters = [{ $($getters)* }]
            ~~> $crate::private_define_interface! {
                $caller
                name = [{ $name }]
                getters = [{ $($getters)* }]
                bounds = [{ $($joined)* }]
            }
        }
    };
    {
        $caller:tt
        name = [{ $name:ident }]
        getters = [{ $($getters:tt)* }]
        bounds = [{ $($bounds:tt)* }]
        trait_def = [{ $($trait_def:tt)* }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ $crate::generate_trait_impl }]
            name = [{ $name }]
            bounds = [{ $($bounds)* }]
            getters = [{ $($getters)* }]
            ~~> $crate::private_define_interface! {
                $caller
                trait_def = [{ $($trait_def)* }]
            }
        }
    };
    {
        $caller:tt
        trait_def = [{ $($trait_def:tt)* }]
        trait_impl = [{ $($trait_impl:tt)* }]
    } => {
        $crate::tt_call::tt_return! {
            $caller
            result = [{ $($trait_def)* $($trait_impl)* }]
        }
    };
}

/// Define a new interface. Used at any layer of your application
/// to declare what dependencies are required by that part of the
/// program.
///
/// Interfaces follow a trait-like syntax, except that they may
/// only contain "getter" methods of a particular form. The names
/// of these methods are for the most part unimportant, but the
/// return types are used to identify dependencies required for
/// a context to implement this interface.
///
/// ## Example
///
/// ```
/// use std::sync::Arc;
///
/// #[derive(Debug)]
/// struct Foo;
///
/// aerosol::define_interface!(
///     TestInterface {
///         fn foo(&self) -> Arc<Foo>;
///     }
/// );
/// ```
///
/// Interfaces may also specify super-traits, which can themselves
/// be interfaces. Interfaces do not need to explicitly list
/// dependencies if they are transitively required by one of their
/// super-traits, but repeating a dependency will still only
/// require it to be provided once.
///
/// ## Example
///
/// ```
/// #![recursion_limit="128"]
/// use std::sync::Arc;
///
/// #[derive(Debug)]
/// struct Foo;
///
/// aerosol::define_interface!(
///     FooInterface {
///         fn foo(&self) -> Arc<Foo>;
///     }
/// );
///
/// aerosol::define_interface!(
///     TestInterface: FooInterface + Clone {}
/// );
/// ```

#[macro_export]
macro_rules! define_interface {
    ($($input:tt)*) => (
        $crate::tt_call::tt_call! {
            macro = [{ $crate::private_define_interface }]
            input = [{ $($input)* }]
        }
    );
}
