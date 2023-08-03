use std::{any::Any, marker::PhantomData, sync::Arc};

use frunk::{hlist::Sculptor, HCons, HNil};

use crate::{
    resource::{unwrap_constructed, unwrap_constructed_hlist, Resource, ResourceList},
    slot::SlotDesc,
    state::Aero,
};

/// Implemented for values which can be constructed from other resources.
pub trait Constructible: Sized + Any + Send + Sync {
    /// Error type for when resource fails to be constructed.
    type Error: Into<anyhow::Error> + Send + Sync;
    /// Construct the resource with the provided application state.
    fn construct(aero: &Aero) -> Result<Self, Self::Error>;

    /// Called after construction with the concrete resource to allow the callee
    /// to provide additional resources. Can be used by eg. an `Arc<Foo>` to also
    /// provide an implementation of `Arc<dyn Bar>`.
    fn after_construction(
        _this: &(dyn Any + Send + Sync),
        _aero: &Aero,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Automatically implemented for values which can be indirectly constructed from other resources.
pub trait IndirectlyConstructible: Sized + Any + Send + Sync {
    /// Error type for when resource fails to be constructed.
    type Error: Into<anyhow::Error> + Send + Sync;
    /// Construct the resource with the provided application state.
    fn construct(aero: &Aero) -> Result<Self, Self::Error>;
    /// Called after construction with the concrete resource to allow the callee
    /// to provide additional resources. Can be used by eg. an `Arc<Foo>` to also
    /// provide an implementation of `Arc<dyn Bar>`.
    fn after_construction(
        _this: &(dyn Any + Send + Sync),
        _aero: &Aero,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<T: Constructible> IndirectlyConstructible for T {
    type Error = T::Error;

    fn construct(aero: &Aero) -> Result<Self, Self::Error> {
        let res = <T as Constructible>::construct(aero)?;
        <T as Constructible>::after_construction(&res, aero)?;
        Ok(res)
    }

    fn after_construction(this: &(dyn Any + Send + Sync), aero: &Aero) -> Result<(), Self::Error> {
        <T as Constructible>::after_construction(this, aero)
    }
}

macro_rules! impl_constructible {
    (<$t:ident>; $($x:ty: $y:expr;)*) => {
        $(
            impl<$t: IndirectlyConstructible> IndirectlyConstructible for $x {
                type Error = $t::Error;

                fn construct(aero: &Aero) -> Result<Self, Self::Error> {
                    let res = $y($t::construct(aero)?);
                    <$t as IndirectlyConstructible>::after_construction(&res, aero)?;
                    Ok(res)
                }

                fn after_construction(this: &(dyn Any + Send + Sync), aero: &Aero) -> Result<(), Self::Error> {
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
/// Do not implement this trait directly, instead implement `Constructible` and ensure
/// the remaining type bounds are met for the automatic implementation of `ConstructibleResource`.
pub trait ConstructibleResource: Resource + IndirectlyConstructible {}
impl<T: Resource + IndirectlyConstructible> ConstructibleResource for T {}

/// Automatically implemented for resource lists where every resource can be constructed.
pub trait ConstructibleResourceList: ResourceList {
    /// Construct every resource in this list in the provided aerosol instance
    fn construct<R: ResourceList>(aero: &Aero<R>) -> anyhow::Result<()>;
}

impl ConstructibleResourceList for HNil {
    fn construct<R: ResourceList>(_aero: &Aero<R>) -> anyhow::Result<()> {
        Ok(())
    }
}

impl<H: ConstructibleResource, T: ConstructibleResourceList> ConstructibleResourceList
    for HCons<H, T>
{
    fn construct<R: ResourceList>(aero: &Aero<R>) -> anyhow::Result<()> {
        aero.try_init::<H>().map_err(Into::into)?;
        T::construct(aero)
    }
}

impl<R: ResourceList> Aero<R> {
    /// Try to get or construct an instance of `T`.
    pub fn try_obtain<T: ConstructibleResource>(&self) -> Result<T, T::Error> {
        match self.try_get_slot() {
            Some(SlotDesc::Filled(x)) => Ok(x),
            Some(SlotDesc::Placeholder) | None => match self.wait_for_slot::<T>(true) {
                Some(x) => Ok(x),
                None => match T::construct(self.as_ref()) {
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
            None => match T::construct(self.as_ref()) {
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

    /// Builder method equivalent to calling `try_init()` but can be chained.
    pub fn try_with_constructed<T: ConstructibleResource>(
        self,
    ) -> Result<Aero<HCons<T, R>>, T::Error> {
        self.try_init::<T>()?;
        Ok(Aero {
            inner: self.inner,
            phantom: PhantomData,
        })
    }

    /// Builder method equivalent to calling `try_init()` but can be chained. Panics if construction fails.
    pub fn with_constructed<T: ConstructibleResource>(self) -> Aero<HCons<T, R>> {
        unwrap_constructed::<T, _>(self.try_with_constructed())
    }

    /// Convert into a different variant of the Aero type. Any missing required resources
    /// will be automatically constructed.
    pub fn try_construct_remaining<R2: ResourceList, I>(self) -> anyhow::Result<Aero<R2>>
    where
        R2: Sculptor<R, I>,
        <R2 as Sculptor<R, I>>::Remainder: ConstructibleResourceList,
    {
        <<R2 as Sculptor<R, I>>::Remainder>::construct(&self)?;
        Ok(Aero {
            inner: self.inner,
            phantom: PhantomData,
        })
    }

    /// Convert into a different variant of the Aero type. Any missing required resources
    /// will be automatically constructed. Panics if construction of any missing resource fails.
    pub fn construct_remaining<R2: ResourceList, I>(self) -> Aero<R2>
    where
        R2: Sculptor<R, I>,
        <R2 as Sculptor<R, I>>::Remainder: ConstructibleResourceList,
    {
        unwrap_constructed_hlist::<<R2 as Sculptor<R, I>>::Remainder, _>(
            self.try_construct_remaining(),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::{convert::Infallible, thread::scope, time::Duration};

    use crate::Aero;

    use super::*;

    #[derive(Debug, Clone)]
    struct Dummy;

    impl Constructible for Dummy {
        type Error = Infallible;

        fn construct(_app_state: &Aero) -> Result<Self, Self::Error> {
            std::thread::sleep(Duration::from_millis(100));
            Ok(Self)
        }
    }

    #[test]
    fn obtain() {
        let state = Aero::new();
        state.obtain::<Dummy>();
    }

    #[test]
    fn obtain_race() {
        let state = Aero::new();
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

        fn construct(aero: &Aero) -> Result<Self, Self::Error> {
            aero.obtain::<Dummy>();
            Ok(Self)
        }
    }

    #[test]
    fn obtain_recursive() {
        let state = Aero::new();
        state.obtain::<DummyRecursive>();
    }

    #[test]
    fn obtain_recursive_race() {
        let state = Aero::new();
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

        fn construct(aero: &Aero) -> Result<Self, Self::Error> {
            aero.obtain::<DummyCyclic>();
            Ok(Self)
        }
    }

    #[test]
    #[should_panic(expected = "Cycle detected")]
    fn obtain_cyclic() {
        let state = Aero::new();
        state.obtain::<DummyCyclic>();
    }

    #[derive(Debug)]
    struct DummyNonClone;

    impl Constructible for DummyNonClone {
        type Error = Infallible;

        fn construct(_app_state: &Aero) -> Result<Self, Self::Error> {
            std::thread::sleep(Duration::from_millis(100));
            Ok(Self)
        }
    }

    #[test]
    fn obtain_non_clone() {
        let state = Aero::new();
        state.obtain::<Arc<DummyNonClone>>();
    }

    trait DummyTrait: Send + Sync {}

    #[derive(Debug)]
    struct DummyImpl;

    impl DummyTrait for DummyImpl {}

    impl Constructible for DummyImpl {
        type Error = Infallible;

        fn construct(_app_state: &Aero) -> Result<Self, Self::Error> {
            Ok(Self)
        }

        fn after_construction(
            this: &(dyn Any + Send + Sync),
            aero: &Aero,
        ) -> Result<(), Self::Error> {
            if let Some(arc) = this.downcast_ref::<Arc<Self>>() {
                aero.insert(arc.clone() as Arc<dyn DummyTrait>)
            }
            Ok(())
        }
    }

    #[test]
    fn obtain_impl() {
        let state = Aero::new();
        state.init::<Arc<DummyImpl>>();
        state.try_get::<Arc<dyn DummyTrait>>().unwrap();
    }

    #[test]
    fn with_constructed() {
        let state = Aero::new().with(42).with_constructed::<Dummy>().with("hi");
        state.get::<Dummy, _>();
    }

    #[test]
    fn construct_remaining() {
        let state: Aero![i32, Dummy, DummyRecursive, &str] =
            Aero::new().with(42).with("hi").construct_remaining();
        state.get::<Dummy, _>();
        state.get::<DummyRecursive, _>();
    }
}
