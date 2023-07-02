use std::{any::Any, sync::Arc};

use async_trait::async_trait;

use crate::{
    resource::{unwrap_constructed, Resource},
    slot::SlotDesc,
    state::Aerosol,
    sync_constructible::Constructible,
};

/// Implemented for values which can be constructed asynchronously from other
/// resources. Requires feature `async`.
#[async_trait]
pub trait AsyncConstructible: Sized + Any + Send + Sync {
    /// Error type for when resource fails to be constructed.
    type Error: Into<anyhow::Error> + Send + Sync;
    /// Construct the resource with the provided application state.
    async fn construct_async(aero: &Aerosol) -> Result<Self, Self::Error>;
    /// Called after construction with the concrete resource to allow the callee
    /// to provide additional resources. Can be used by eg. an `Arc<Foo>` to also
    /// provide an implementation of `Arc<dyn Bar>`.
    async fn after_construction_async(
        _this: &(dyn Any + Send + Sync),
        _aero: &Aerosol,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[async_trait]
impl<T: Constructible> AsyncConstructible for T {
    type Error = <T as Constructible>::Error;
    async fn construct_async(aero: &Aerosol) -> Result<Self, Self::Error> {
        Self::construct(aero)
    }
    async fn after_construction_async(
        this: &(dyn Any + Send + Sync),
        aero: &Aerosol,
    ) -> Result<(), Self::Error> {
        Self::after_construction(this, aero)
    }
}

/// Automatically implemented for values which can be indirectly asynchronously constructed from other resources.
/// Requires feature `async`.
#[async_trait]
pub trait IndirectlyAsyncConstructible: Sized + Any + Send + Sync {
    /// Error type for when resource fails to be constructed.
    type Error: Into<anyhow::Error> + Send + Sync;
    /// Construct the resource with the provided application state.
    async fn construct_async(aero: &Aerosol) -> Result<Self, Self::Error>;
    /// Called after construction with the concrete resource to allow the callee
    /// to provide additional resources. Can be used by eg. an `Arc<Foo>` to also
    /// provide an implementation of `Arc<dyn Bar>`.
    async fn after_construction_async(
        _this: &(dyn Any + Send + Sync),
        _aero: &Aerosol,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[async_trait]
impl<T: AsyncConstructible> IndirectlyAsyncConstructible for T {
    type Error = T::Error;

    async fn construct_async(aero: &Aerosol) -> Result<Self, Self::Error> {
        let res = <T as AsyncConstructible>::construct_async(aero).await?;
        <T as AsyncConstructible>::after_construction_async(&res, aero).await?;
        Ok(res)
    }
    async fn after_construction_async(
        this: &(dyn Any + Send + Sync),
        aero: &Aerosol,
    ) -> Result<(), Self::Error> {
        <T as AsyncConstructible>::after_construction_async(this, aero).await
    }
}

macro_rules! impl_async_constructible {
    (<$t:ident>; $($x:ty: $y:expr;)*) => {
        $(
            #[async_trait]
            impl<$t: IndirectlyAsyncConstructible> IndirectlyAsyncConstructible for $x {
                type Error = $t::Error;

                async fn construct_async(aero: &Aerosol) -> Result<Self, Self::Error> {
                    let res = $y($t::construct_async(aero).await?);
                    <$t as IndirectlyAsyncConstructible>::after_construction_async(&res, aero).await?;
                    Ok(res)
                }

                async fn after_construction_async(this: &(dyn Any + Send + Sync), aero: &Aerosol) -> Result<(), Self::Error> {
                    <$t as IndirectlyAsyncConstructible>::after_construction_async(this, aero).await
                }
            }
        )*
    };
}
impl_async_constructible! {
    <T>;
    Arc<T>: Arc::new;
    std::sync::Mutex<T>: std::sync::Mutex::new;
    parking_lot::Mutex<T>: parking_lot::Mutex::new;
    std::sync::RwLock<T>: std::sync::RwLock::new;
    parking_lot::RwLock<T>: parking_lot::RwLock::new;
}

/// Implemented for resources which can be asynchronously constructed from other resources. Requires feature `async`.
pub trait AsyncConstructibleResource: Resource + IndirectlyAsyncConstructible {}
impl<T: Resource + IndirectlyAsyncConstructible> AsyncConstructibleResource for T {}

impl Aerosol {
    /// Try to get or construct an instance of `T` asynchronously. Requires feature `async`.
    pub async fn try_obtain_async<T: AsyncConstructibleResource>(&self) -> Result<T, T::Error> {
        match self.try_get_slot() {
            Some(SlotDesc::Filled(x)) => Ok(x),
            Some(SlotDesc::Placeholder) | None => match self.wait_for_slot_async::<T>(true).await {
                Some(x) => Ok(x),
                None => match T::construct_async(self).await {
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
    /// Get or construct an instance of `T` asynchronously. Panics if unable. Requires feature `async`.
    pub async fn obtain_async<T: AsyncConstructibleResource>(&self) -> T {
        unwrap_constructed::<T, _>(self.try_obtain_async::<T>().await)
    }
    /// Try to initialize an instance of `T` asynchronously. Does nothing if `T` is already initialized.
    pub async fn try_init_async<T: AsyncConstructibleResource>(&self) -> Result<(), T::Error> {
        match self.wait_for_slot_async::<T>(true).await {
            Some(_) => Ok(()),
            None => match T::construct_async(self).await {
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
    /// Initialize an instance of `T` asynchronously. Does nothing if `T` is already initialized. Panics if unable.
    pub async fn init_async<T: AsyncConstructibleResource>(&self) {
        unwrap_constructed::<T, _>(self.try_init_async::<T>().await)
    }
}

#[cfg(test)]
mod tests {
    use std::{convert::Infallible, time::Duration};

    use super::*;

    #[derive(Debug, Clone)]
    struct Dummy;

    #[async_trait]
    impl AsyncConstructible for Dummy {
        type Error = Infallible;

        async fn construct_async(_app_state: &Aerosol) -> Result<Self, Self::Error> {
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok(Self)
        }
    }

    #[tokio::test]
    async fn obtain() {
        let state = Aerosol::new();
        state.obtain_async::<Dummy>().await;
    }

    #[tokio::test]
    async fn obtain_race() {
        let state = Aerosol::new();
        let mut handles = Vec::new();
        for _ in 0..100 {
            let state = state.clone();
            handles.push(tokio::spawn(async move {
                state.obtain_async::<Dummy>().await;
            }));
        }
        for handle in handles {
            handle.await.unwrap();
        }
    }

    #[derive(Debug, Clone)]
    struct DummyRecursive;

    #[async_trait]
    impl AsyncConstructible for DummyRecursive {
        type Error = Infallible;

        async fn construct_async(aero: &Aerosol) -> Result<Self, Self::Error> {
            aero.obtain_async::<Dummy>().await;
            Ok(Self)
        }
    }

    #[tokio::test]
    async fn obtain_recursive() {
        let state = Aerosol::new();
        state.obtain_async::<DummyRecursive>().await;
    }

    #[tokio::test]
    async fn obtain_recursive_race() {
        let state = Aerosol::new();
        let mut handles = Vec::new();
        for _ in 0..100 {
            let state = state.clone();
            handles.push(tokio::spawn(async move {
                state.obtain_async::<DummyRecursive>().await;
            }));
        }
    }

    #[derive(Debug, Clone)]
    struct DummyCyclic;

    #[async_trait]
    impl AsyncConstructible for DummyCyclic {
        type Error = Infallible;

        async fn construct_async(aero: &Aerosol) -> Result<Self, Self::Error> {
            aero.obtain_async::<DummyCyclic>().await;
            Ok(Self)
        }
    }

    #[tokio::test]
    #[should_panic(expected = "Cycle detected")]
    async fn obtain_cyclic() {
        let state = Aerosol::new();
        state.obtain_async::<DummyCyclic>().await;
    }

    #[derive(Debug, Clone)]
    struct DummySync;

    impl Constructible for DummySync {
        type Error = Infallible;

        fn construct(_app_state: &Aerosol) -> Result<Self, Self::Error> {
            std::thread::sleep(Duration::from_millis(100));
            Ok(Self)
        }
    }

    #[derive(Debug, Clone)]
    struct DummySyncRecursive;

    #[async_trait]
    impl AsyncConstructible for DummySyncRecursive {
        type Error = Infallible;

        async fn construct_async(aero: &Aerosol) -> Result<Self, Self::Error> {
            aero.obtain_async::<DummySync>().await;
            Ok(Self)
        }
    }

    #[tokio::test]
    async fn obtain_sync_recursive() {
        let state = Aerosol::new();
        state.obtain_async::<DummySyncRecursive>().await;
    }

    #[tokio::test]
    async fn obtain_sync_recursive_race() {
        let state = Aerosol::new();
        let mut handles = Vec::new();
        for _ in 0..100 {
            let state = state.clone();
            handles.push(tokio::spawn(async move {
                state.obtain_async::<DummySyncRecursive>().await;
            }));
        }
    }

    #[derive(Debug)]
    struct DummyNonClone;

    #[async_trait]
    impl AsyncConstructible for DummyNonClone {
        type Error = Infallible;

        async fn construct_async(_app_state: &Aerosol) -> Result<Self, Self::Error> {
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok(Self)
        }
    }

    #[tokio::test]
    async fn obtain_non_clone() {
        let state = Aerosol::new();
        state.obtain_async::<Arc<DummyNonClone>>().await;
    }

    trait DummyTrait: Send + Sync {}

    #[derive(Debug)]
    struct DummyImpl;

    impl DummyTrait for DummyImpl {}

    #[async_trait]
    impl AsyncConstructible for DummyImpl {
        type Error = Infallible;

        async fn construct_async(_app_state: &Aerosol) -> Result<Self, Self::Error> {
            Ok(Self)
        }

        async fn after_construction_async(
            this: &(dyn Any + Send + Sync),
            aero: &Aerosol,
        ) -> Result<(), Self::Error> {
            if let Some(arc) = this.downcast_ref::<Arc<Self>>() {
                aero.insert(arc.clone() as Arc<dyn DummyTrait>)
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn obtain_impl() {
        let state = Aerosol::new();
        state.init_async::<Arc<DummyImpl>>().await;
        state.get_async::<Arc<dyn DummyTrait>>().await;
    }
}
