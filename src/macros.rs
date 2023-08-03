/// Define a custom `Aero` alias with a specific set of required types
///
/// Example usage:
/// ```rust
/// use aerosol::Aero;
///
/// type AppState = Aero![&'static str, i32, bool];
/// ```
#[macro_export]
macro_rules! Aero {
    ($($tok:tt)*) => {
        $crate::Aero<$crate::frunk::HList![$($tok)*]>
    };
}
