#[doc(hidden)]
#[macro_export(local_inner_macros)]
macro_rules! parse_bound {
    {
        $caller:tt
        input = [{ $($input:tt)* }]
    } => {
        parse_bound! {
            $caller
            rest = [{ $($input)* }]
        }
    };
    {
        $caller:tt
        $(bound = [{ $($bound:tt)* }])*
        rest = [{ $($rest:tt)* }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ $crate::tt_call::parse_type }]
            input = [{ $($rest)* }]
            ~~> parse_bound! {
                $caller
                $(bound = [{ $($bound)* }])*
            }
        }
    };
    {
        $caller:tt
        $(bound = [{ $($bound:tt)* }])*
        type = [{ $($type:tt)* }]
        rest = [{ + $($rest:tt)* }]
    } => {
        parse_bound! {
            $caller
            $(bound = [{ $($bound)* }])*
            bound = [{ $($type)* }]
            rest = [{ $($rest)* }]
        }
    };
    {
        $caller:tt
        $(bound = [{ $($bound:tt)* }])*
        type = [{ $($type:tt)* }]
        rest = [{ $($rest:tt)* }]
    } => {
        $crate::tt_call::tt_return! {
            $caller
            $(bound = [{ $($bound)* }])*
            bound = [{ $($type)* }]
            rest = [{ $($rest)* }]
        }
    };
}

#[doc(hidden)]
#[macro_export(local_inner_macros)]
macro_rules! parse_trait_def {
    {
        $caller:tt
        input = [{ $name:ident { $($body:tt)* } }]
    } => {
        $crate::tt_call::tt_return! {
            $caller
            name = [{ $name }]
            body = [{ $($body)* }]
        }
    };
    {
        $caller:tt
        input = [{ $name:ident: $($rest:tt)* }]
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ parse_bound }]
            input = [{ $($rest)* }]
            ~~> parse_trait_def! {
                $caller
                name = [{ $name }]
            }
        }
    };
    {
        $caller:tt
        name = [{ $name:ident }]
        $(bound = [{ $($bound:tt)* }])*
        rest = [{ { $($body:tt)* } }]
    } => {
        $crate::tt_call::tt_return! {
            $caller
            name = [{ $name }]
            body = [{ $($body)* }]
            $(bound = [{ $($bound)* }])*
        }
    };
}
