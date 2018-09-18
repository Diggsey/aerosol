
#[macro_export(local_inner_macros)]
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


#[macro_export(local_inner_macros)]
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

#[macro_export(local_inner_macros)]
macro_rules! private_define_interface {
    {
        $caller:tt
        input = [{ $($input:tt)* }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ parse_trait_def }]
            input = [{ $($input)* }]
            ~~> private_define_interface! {
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
            macro = [{ join }]
            sep = [{ + }]
            $(item = [{ $($bound)* }])*
            $(item = [{ $crate::Provide<$t> }])*
            ~~> private_define_interface! {
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
            macro = [{ generate_trait_def }]
            name = [{ $name }]
            bounds = [{ $($joined)* }]
            getters = [{ $($getters)* }]
            ~~> private_define_interface! {
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
            macro = [{ generate_trait_impl }]
            name = [{ $name }]
            bounds = [{ $($bounds)* }]
            getters = [{ $($getters)* }]
            ~~> private_define_interface! {
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

#[macro_export(local_inner_macros)]
macro_rules! define_interface {
    ($($input:tt)*) => (
        $crate::tt_call::tt_call! {
            macro = [{ private_define_interface }]
            input = [{ $($input)* }]
        }
    );
}
