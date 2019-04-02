
#[doc(hidden)]
#[macro_export(local_inner_macros)]
macro_rules! private_define_context {
    {
        $caller:tt
        input = [{
            $name:ident {
                $($body:tt)*
            }
        }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ private_define_context }]
            rest = [{ $($body)* }]
            ~~> private_define_context! {
                $caller
                name = [{ $name }]
            }
        }
    };
    {
        $caller:tt
        name = [{ $name:ident }]
        $(auto_field = [{ $auto_field:ident, $auto_t:ty, $factory:ty, ($($f_args:ident,)*) }])*
        $(field = [{ $field:ident, $t:ty }])*
    } => {
        $crate::tt_call::tt_return! {
            $caller
            result = [{
                #[derive(Clone, Debug)]
                struct $name {
                    $($auto_field: $auto_t,)*
                    $($field: $t,)*
                }

                impl $name {
                    fn new($($field: $t,)*) -> Result<Self, $crate::failure::Error> {
                        $(
                            let $auto_field = <$factory as $crate::Factory<_>>::build(($($f_args.clone(),)*))?;
                        )*
                        Ok(Self {
                            $($auto_field,)*
                            $($field,)*
                        })
                    }
                }

                $(
                    impl $crate::Provide<$auto_t> for $name {
                        fn provide(&self) -> $auto_t {
                            self.$auto_field.clone()
                        }
                    }
                    impl $crate::ProvideWith<$auto_t> for $name {
                        fn provide_with<E, F: FnOnce($auto_t) -> Result<$auto_t, E>>(&self, f: F) -> Result<Self, E> {
                            let mut result = self.clone();
                            result.$auto_field = f(result.$auto_field)?;
                            Ok(result)
                        }
                    }
                )*

                $(
                    impl $crate::Provide<$t> for $name {
                        fn provide(&self) -> $t {
                            self.$field.clone()
                        }
                    }
                    impl $crate::ProvideWith<$t> for $name {
                        fn provide_with<E, F: FnOnce($t) -> Result<$t, E>>(&self, f: F) -> Result<Self, E> {
                            let mut result = self.clone();
                            result.$field = f(result.$field)?;
                            Ok(result)
                        }
                    }
                )*
            }]
        }
    };
    {
        $caller:tt
        $(auto_field = [{ $($auto_field:tt)* }])*
        $(field = [{ $($field:tt)* }])*
        rest = [{ $field_name:ident: $t:ty [ ($($f_args:ident),*) $factory:ty ], $($rest:tt)* }]
    } => {
        private_define_context! {
            $caller
            $(auto_field = [{ $($auto_field)* }])*
            auto_field = [{ $field_name, $t, $factory, ($($f_args,)*) }]
            $(field = [{ $($field)* }])*
            rest = [{ $($rest)* }]
        }
    };
    {
        $caller:tt
        $(auto_field = [{ $($auto_field:tt)* }])*
        $(field = [{ $($field:tt)* }])*
        rest = [{ $field_name:ident: $t:ty [ ($($f_args:ident),*) $factory:ty ] }]
    } => {
        private_define_context! {
            $caller
            $(auto_field = [{ $($auto_field)* }])*
            auto_field = [{ $field_name, $t, $factory, ($($f_args,)*) }]
            $(field = [{ $($field)* }])*
            rest = [{ }]
        }
    };
    {
        $caller:tt
        $(auto_field = [{ $($auto_field:tt)* }])*
        $(field = [{ $($field:tt)* }])*
        rest = [{ $field_name:ident: $t:ty [ $factory:ty ], $($rest:tt)* }]
    } => {
        private_define_context! {
            $caller
            $(auto_field = [{ $($auto_field)* }])*
            auto_field = [{ $field_name, $t, $factory, () }]
            $(field = [{ $($field)* }])*
            rest = [{ $($rest)* }]
        }
    };
    {
        $caller:tt
        $(auto_field = [{ $($auto_field:tt)* }])*
        $(field = [{ $($field:tt)* }])*
        rest = [{ $field_name:ident: $t:ty [ $factory:ty ] }]
    } => {
        private_define_context! {
            $caller
            $(auto_field = [{ $($auto_field)* }])*
            auto_field = [{ $field_name, $t, $factory, () }]
            $(field = [{ $($field)* }])*
            rest = [{ }]
        }
    };
    {
        $caller:tt
        $(auto_field = [{ $($auto_field:tt)* }])*
        $(field = [{ $($field:tt)* }])*
        rest = [{ $field_name:ident: $t:ty, $($rest:tt)* }]
    } => {
        private_define_context! {
            $caller
            $(auto_field = [{ $($auto_field)* }])*
            $(field = [{ $($field)* }])*
            field = [{ $field_name, $t }]
            rest = [{ $($rest)* }]
        }
    };
    {
        $caller:tt
        $(auto_field = [{ $($auto_field:tt)* }])*
        $(field = [{ $($field:tt)* }])*
        rest = [{ $field_name:ident: $t:ty }]
    } => {
        private_define_context! {
            $caller
            $(auto_field = [{ $($auto_field)* }])*
            $(field = [{ $($field)* }])*
            field = [{ $field_name, $t }]
            rest = [{ }]
        }
    };
    {
        $caller:tt
        $(auto_field = [{ $($auto_field:tt)* }])*
        $(field = [{ $($field:tt)* }])*
        rest = [{ }]
    } => {
        $crate::tt_call::tt_return! {
            $caller
            $(auto_field = [{ $($auto_field)* }])*
            $(field = [{ $($field)* }])*
        }
    };
}

/// Define a new context. Typically used at the top level of an
/// application to contain the full set of requried dependencies.
/// 
/// Contexts follow a struct-like syntax, although the names of
/// fields are for the most part unimportant.
/// 
/// Contexts automatically implement all applicable interfaces.
/// An interface is applicable if all of the dependencies
/// required by that interface are present in the context.
/// 
/// Dependencies are identified by *type*, not by the field name.
/// Contexts may not contain two fields of the same type. Instead
/// use new-type wrappers to distinguish similar dependencies.
/// 
/// Types used in a context must implement `Clone + Debug`, and
/// `Clone` should be a cheap operation. For this reason it is usual
/// to wrap dependencies in an `Rc` or `Arc`.
/// 
/// A constructor function will be automatically implemented
/// for contexts, with one parameter for each dependency, to be
/// provided in the same order as when the context is defined.
/// 
/// ## Example
/// 
/// ```
/// use std::sync::Arc;
/// 
/// #[derive(Debug)]
/// struct Foo;
/// #[derive(Debug)]
/// struct Bar;
/// 
/// aerosol::define_context!(
///     TestContext {
///         foo: Arc<Foo>,
///         bar: Arc<Bar>,
///     }
/// );
/// 
/// fn main() {
///     TestContext::new(
///         Arc::new(Foo),
///         Arc::new(Bar),
///     );
/// }
/// ```
/// 
/// It is also possible to define a factory type to enable
/// dependencies to be automatically created.
/// 
/// When a factory is specified for a dependency, it will be
/// omitted from the parameter list required by the context's
/// constructor. Instead, the constructor will call the `build`
/// method on the specified factory.
/// 
/// To conditionally use a factory, or use different factories
/// for the same dependency, define separate contexts, or
/// call the factory manually and pass the result to the
/// context's constructor in the normal way.
/// 
/// ## Example
/// 
/// ```
/// use std::sync::Arc;
/// use failure;
/// 
/// #[derive(Debug)]
/// struct Foo;
/// #[derive(Debug)]
/// struct Bar;
/// 
/// struct FooFactory;
/// impl aerosol::Factory for FooFactory {
///     type Object = Arc<Foo>;
///     fn build(_: ()) -> Result<Arc<Foo>, failure::Error> { Ok(Arc::new(Foo)) }
/// }
/// 
/// aerosol::define_context!(
///     TestContext {
///         foo: Arc<Foo> [FooFactory],
///         bar: Arc<Bar>,
///     }
/// );
/// 
/// fn main() {
///     TestContext::new(
///         Arc::new(Bar),
///     );
/// }
/// ```
/// 
/// 
#[macro_export(local_inner_macros)]
macro_rules! define_context {
    ($($input:tt)*) => (
        $crate::tt_call::tt_call! {
            macro = [{ private_define_context }]
            input = [{ $($input)* }]
        }
    );
}
