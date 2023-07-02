use std::{any::Any, sync::Arc};

use crate::{
    resource::{unwrap_constructed, Resource},
    slot::SlotDesc,
    state::Aerosol,
};

/// Implemented for values which can be constructed from other resources.
pub trait Constructible: Sized + Any + Send + Sync {
    /// Error type for when resource fails to be constructed.
    type Error: Into<anyhow::Error> + Send + Sync;
    /// Construct the resource with the provided application state.
    fn construct(aero: &Aerosol) -> Result<Self, Self::Error>;

    /// Called after construction with the concrete resource to allow the callee
    /// to provide additional resources. Can be used by eg. an `Arc<Foo>` to also
    /// provide an implementation of `Arc<dyn Bar>`.
    fn after_construction(
        _this: &(dyn Any + Send + Sync),
        _aero: &Aerosol,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Automatically implemented for values which can be indirectly constructed from other resources.
pub trait IndirectlyConstructible: Sized + Any + Send + Sync {
    /// Error type for when resource fails to be constructed.
    type Error: Into<anyhow::Error> + Send + Sync;
    /// Construct the resource with the provided application state.
    fn construct(aero: &Aerosol) -> Result<Self, Self::Error>;
    /// Called after construction with the concrete resource to allow the callee
    /// to provide additional resources. Can be used by eg. an `Arc<Foo>` to also
    /// provide an implementation of `Arc<dyn Bar>`.
    fn after_construction(
        _this: &(dyn Any + Send + Sync),
        _aero: &Aerosol,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<T: Constructible> IndirectlyConstructible for T {
    type Error = T::Error;

    fn construct(aero: &Aerosol) -> Result<Self, Self::Error> {
        let res = <T as Constructible>::construct(aero)?;
        <T as Constructible>::after_construction(&res, aero)?;
        Ok(res)
    }

    fn after_construction(
        this: &(dyn Any + Send + Sync),
        aero: &Aerosol,
    ) -> Result<(), Self::Error> {
        <T as Constructible>::after_construction(this, aero)
    }
}

macro_rules! impl_constructible {
    (<$t:ident>; $($x:ty: $y:expr;)*) => {
        $(
            impl<$t: IndirectlyConstructible> IndirectlyConstructible for $x {
                type Error = $t::Error;

                fn construct(aero: &Aerosol) -> Result<Self, Self::Error> {
                    let res = $y($t::construct(aero)?);
                    <$t as IndirectlyConstructible>::after_construction(&res, aero)?;
                    Ok(res)
                }

                fn after_construction(this: &(dyn Any + Send + Sync), aero: &Aerosol) -> Result<(), Self::Error> {
                    <$t as IndirectlyConstructible>::after_construction(this, aero)
                }
            }
        )*
    };
}
impl_constructible! {
    <T>;
    Arc<T>: Arc::new;
    std::sync::Mutex<T>: std::sync::Mutex::new;
    parking_lot::Mutex<T>: parking_lot::Mutex::new;
    std::sync::RwLock<T>: std::sync::RwLock::new;
    parking_lot::RwLock<T>: parking_lot::RwLock::new;
}

/// Implemented for resources which can be constructed from other resources.
pub trait ConstructibleResource: Resource + IndirectlyConstructible {}
impl<T: Resource + IndirectlyConstructible> ConstructibleResource for T {}

impl Aerosol {
    /// Try to get or construct an instance of `T`.
    pub fn try_obtain<T: ConstructibleResource>(&self) -> Result<T, T::Error> {
        match self.try_get_slot() {
            Some(SlotDesc::Filled(x)) => Ok(x),
            Some(SlotDesc::Placeholder) | None => match self.wait_for_slot::<T>(true) {
                Some(x) => Ok(x),
                None => match T::construct(self) {
                    Ok(x) => {
                        self.fill_placeholder::<T>(x.clone());
                        Ok(x)
                    }
                    Err(e) => {
                        self.clear_placeholder::<T>();
                        Err(e)
                    }
                },
            },
        }
    }
    /// Get or construct an instance of `T`. Panics if unable.
    pub fn obtain<T: ConstructibleResource>(&self) -> T {
        unwrap_constructed::<T, _>(self.try_obtain::<T>())
    }
    /// Try to initialize an instance of `T`. Does nothing if `T` is already initialized.
    pub fn try_init<T: ConstructibleResource>(&self) -> Result<(), T::Error> {
        match self.wait_for_slot::<T>(true) {
            Some(_) => Ok(()),
            None => match T::construct(self) {
                Ok(x) => {
                    self.fill_placeholder::<T>(x);
                    Ok(())
                }
                Err(e) => {
                    self.clear_placeholder::<T>();
                    Err(e)
                }
            },
        }
    }
    /// Initialize an instance of `T`. Does nothing if `T` is already initialized. Panics if unable.
    pub fn init<T: ConstructibleResource>(&self) {
        unwrap_constructed::<T, _>(self.try_init::<T>())
    }
}

#[cfg(test)]
mod tests {
    use std::{convert::Infallible, thread::scope, time::Duration};

    use super::*;

    #[derive(Debug, Clone)]
    struct Dummy;

    impl Constructible for Dummy {
        type Error = Infallible;

        fn construct(_app_state: &Aerosol) -> Result<Self, Self::Error> {
            std::thread::sleep(Duration::from_millis(100));
            Ok(Self)
        }
    }

    #[test]
    fn obtain() {
        let state = Aerosol::new();
        state.obtain::<Dummy>();
    }

    #[test]
    fn obtain_race() {
        let state = Aerosol::new();
        scope(|s| {
            for _ in 0..100 {
                s.spawn(|| state.obtain::<Dummy>());
            }
        });
    }

    #[derive(Debug, Clone)]
    struct DummyRecursive;

    impl Constructible for DummyRecursive {
        type Error = Infallible;

        fn construct(aero: &Aerosol) -> Result<Self, Self::Error> {
            aero.obtain::<Dummy>();
            Ok(Self)
        }
    }

    #[test]
    fn obtain_recursive() {
        let state = Aerosol::new();
        state.obtain::<DummyRecursive>();
    }

    #[test]
    fn obtain_recursive_race() {
        let state = Aerosol::new();
        scope(|s| {
            for _ in 0..100 {
                s.spawn(|| state.obtain::<DummyRecursive>());
            }
        });
    }

    #[derive(Debug, Clone)]
    struct DummyCyclic;

    impl Constructible for DummyCyclic {
        type Error = Infallible;

        fn construct(aero: &Aerosol) -> Result<Self, Self::Error> {
            aero.obtain::<DummyCyclic>();
            Ok(Self)
        }
    }

    #[test]
    #[should_panic(expected = "Cycle detected")]
    fn obtain_cyclic() {
        let state = Aerosol::new();
        state.obtain::<DummyCyclic>();
    }

    #[derive(Debug)]
    struct DummyNonClone;

    impl Constructible for DummyNonClone {
        type Error = Infallible;

        fn construct(_app_state: &Aerosol) -> Result<Self, Self::Error> {
            std::thread::sleep(Duration::from_millis(100));
            Ok(Self)
        }
    }

    #[test]
    fn obtain_non_clone() {
        let state = Aerosol::new();
        state.obtain::<Arc<DummyNonClone>>();
    }

    trait DummyTrait: Send + Sync {}

    #[derive(Debug)]
    struct DummyImpl;

    impl DummyTrait for DummyImpl {}

    impl Constructible for DummyImpl {
        type Error = Infallible;

        fn construct(_app_state: &Aerosol) -> Result<Self, Self::Error> {
            Ok(Self)
        }

        fn after_construction(
            this: &(dyn Any + Send + Sync),
            aero: &Aerosol,
        ) -> Result<(), Self::Error> {
            if let Some(arc) = this.downcast_ref::<Arc<Self>>() {
                aero.insert(arc.clone() as Arc<dyn DummyTrait>)
            }
            Ok(())
        }
    }

    #[test]
    fn obtain_impl() {
        let state = Aerosol::new();
        state.init::<Arc<DummyImpl>>();
        state.get::<Arc<dyn DummyTrait>>();
    }
}
