#[doc(hidden)]
#[macro_export(local_inner_macros)]
macro_rules! join {
    {
        $caller:tt
        sep = [{ $($sep:tt)* }]
    } => {
        $crate::tt_call::tt_return! {
            $caller
            joined = [{ }]
        }
    };
    {
        $caller:tt
        sep = [{ $($sep:tt)* }]
        item = [{ $($head:tt)* }]
        $(
            item = [{ $($tail:tt)* }]
        )+
    } => {
        $crate::tt_call::tt_call! {
            macro = [{ $crate::join }]
            sep = [{ $($sep)* }]
            $(
                item = [{ $($tail)* }]
            )+
            ~~> $crate::join! {
                $caller
                prepend = [{ $($head)* $($sep)* }]
            }
        }
    };
    {
        $caller:tt
        sep = [{ $($sep:tt)* }]
        item = [{ $($head:tt)* }]
    } => {
        $crate::tt_call::tt_return! {
            $caller
            joined = [{ $($head)* }]
        }
    };
    {
        $caller:tt
        prepend = [{ $($prepend:tt)* }]
        joined = [{ $($joined:tt)* }]
    } => {
        $crate::tt_call::tt_return! {
            $caller
            joined = [{ $($prepend)* $($joined)* }]
        }
    };
}
