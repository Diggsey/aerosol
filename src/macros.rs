/// Define a custom `Aerosol` alias with a specific set of required types
#[macro_export]
macro_rules! Aero {
    ($($tok:tt)*) => {
        $crate::Aerosol<$crate::frunk::HList![$($tok)*]>
    };
}
