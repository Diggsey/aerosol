#[doc(hidden)]
#[macro_export]
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
        item = [{ $($first:tt)* }]
    } => {
        $crate::tt_call::tt_return! {
            $caller
            joined = [{ $($first)* }]
        }
    };
    {
        $caller:tt
        sep = [{ $($sep:tt)* }]
        item = [{ $($first:tt)* }]
        item = [{ $($second:tt)* }]
        $(
            item = [{ $($rest:tt)* }]
        )*
    } => {
        $crate::join! {
            $caller
            sep = [{ $($sep)* }]
            item = [{ $($first)* $($sep)* $($second)* }]
            $(
                item = [{ $($rest)* }]
            )*
        }
    };
}

#[test]
fn test_join() {
    use tt_call::*;

    let s = tt_call! {
        macro = [{ join }]
        sep = [{ .chars().rev().collect::<String>() + "_" + & }]
        item = [{ "first   " }]
        item = [{ "second  ".trim() }]
        item = [{ "third   " }]
    };

    assert_eq!(s, "   tsrif_dnoces_third   ");
}
