use std::error::Error;

use crate::{
    resource::{unwrap_constructed, Resource},
    slot::SlotDesc,
    state::Aerosol,
};

/// Implemented for resources which can be constructed from other resources.
pub trait ConstructibleResource: Resource {
    /// Error type for when resource fails to be constructed.
    type Error: Error + Send + Sync;
    /// Construct the resource with the provided application state.
    fn construct(aero: &Aerosol) -> Result<Self, Self::Error>;
}

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
        unwrap_constructed(self.try_obtain::<T>())
    }
}

#[cfg(test)]
mod tests {
    use std::{convert::Infallible, thread::scope, time::Duration};

    use super::*;

    #[derive(Debug, Clone)]
    struct Dummy;

    impl ConstructibleResource for Dummy {
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

    impl ConstructibleResource for DummyRecursive {
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

    impl ConstructibleResource for DummyCyclic {
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
}
