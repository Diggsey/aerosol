
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
        $(auto_field = [{ $auto_field:ident, $auto_t:ty, $factory:ty }])*
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
                        Ok(Self {
                            $($auto_field: <$factory as $crate::Factory>::build()?,)*
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
        rest = [{ $field_name:ident: $t:ty [ $factory:ty ], $($rest:tt)* }]
    } => {
        private_define_context! {
            $caller
            $(auto_field = [{ $($auto_field)* }])*
            auto_field = [{ $field_name, $t, $factory }]
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
            auto_field = [{ $field_name, $t, $factory }]
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

#[macro_export(local_inner_macros)]
macro_rules! define_context {
    ($($input:tt)*) => (
        $crate::tt_call::tt_call! {
            macro = [{ private_define_context }]
            input = [{ $($input)* }]
        }
    );
}
