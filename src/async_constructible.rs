use std::{any::Any, marker::PhantomData, sync::Arc};

use async_trait::async_trait;
use frunk::{hlist::Sculptor, HCons, HNil};

use crate::{
    resource::{unwrap_constructed, unwrap_constructed_hlist, Resource, ResourceList},
    slot::SlotDesc,
    state::Aero,
    sync_constructible::Constructible,
};

/// Implemented for values which can be constructed asynchronously from other
/// resources. Requires feature `async`.
#[async_trait]
pub trait AsyncConstructible: Sized + Any + Send + Sync {
    /// Error type for when resource fails to be constructed.
    type Error: Into<anyhow::Error> + Send + Sync;
    /// Construct the resource with the provided application state.
    async fn construct_async(aero: &Aero) -> Result<Self, Self::Error>;
    /// Called after construction with the concrete resource to allow the callee
    /// to provide additional resources. Can be used by eg. an `Arc<Foo>` to also
    /// provide an implementation of `Arc<dyn Bar>`.
    async fn after_construction_async(
        _this: &(dyn Any + Send + Sync),
        _aero: &Aero,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[async_trait]
impl<T: Constructible> AsyncConstructible for T {
    type Error = <T as Constructible>::Error;
    async fn construct_async(aero: &Aero) -> Result<Self, Self::Error> {
        Self::construct(aero)
    }
    async fn after_construction_async(
        this: &(dyn Any + Send + Sync),
        aero: &Aero,
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
    async fn construct_async(aero: &Aero) -> Result<Self, Self::Error>;
    /// Called after construction with the concrete resource to allow the callee
    /// to provide additional resources. Can be used by eg. an `Arc<Foo>` to also
    /// provide an implementation of `Arc<dyn Bar>`.
    async fn after_construction_async(
        _this: &(dyn Any + Send + Sync),
        _aero: &Aero,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[async_trait]
impl<T: AsyncConstructible> IndirectlyAsyncConstructible for T {
    type Error = T::Error;

    async fn construct_async(aero: &Aero) -> Result<Self, Self::Error> {
        let res = <T as AsyncConstructible>::construct_async(aero).await?;
        <T as AsyncConstructible>::after_construction_async(&res, aero).await?;
        Ok(res)
    }
    async fn after_construction_async(
        this: &(dyn Any + Send + Sync),
        aero: &Aero,
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

                async fn construct_async(aero: &Aero) -> Result<Self, Self::Error> {
                    let res = $y($t::construct_async(aero).await?);
                    <$t as IndirectlyAsyncConstructible>::after_construction_async(&res as &(dyn Any + Send + Sync), aero).await?;
                    Ok(res)
                }

                async fn after_construction_async(this: &(dyn Any + Send + Sync), aero: &Aero) -> Result<(), Self::Error> {
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
/// Do not implement this trait directly, instead implement `AsyncConstructible` and ensure
/// the remaining type bounds are met for the automatic implementation of `AsyncConstructibleResource`.
pub trait AsyncConstructibleResource: Resource + IndirectlyAsyncConstructible {}
impl<T: Resource + IndirectlyAsyncConstructible> AsyncConstructibleResource for T {}

/// Automatically implemented for resource lists where every resource can be asynchronously constructed.
#[async_trait]
pub trait AsyncConstructibleResourceList: ResourceList {
    /// Construct every resource in this list in the provided aerosol instance
    async fn construct_async<R: ResourceList>(aero: &Aero<R>) -> anyhow::Result<()>;
}

#[async_trait]
impl AsyncConstructibleResourceList for HNil {
    async fn construct_async<R: ResourceList>(_aero: &Aero<R>) -> anyhow::Result<()> {
        Ok(())
    }
}

#[async_trait]
impl<H: AsyncConstructibleResource, T: AsyncConstructibleResourceList>
    AsyncConstructibleResourceList for HCons<H, T>
{
    async fn construct_async<R: ResourceList>(aero: &Aero<R>) -> anyhow::Result<()> {
        aero.try_init_async::<H>().await.map_err(Into::into)?;
        T::construct_async(aero).await
    }
}

impl<R: ResourceList> Aero<R> {
    /// Try to get or construct an instance of `T` asynchronously. Requires feature `async`.
    pub async fn try_obtain_async<T: AsyncConstructibleResource>(&self) -> Result<T, T::Error> {
        match self.try_get_slot() {
            Some(SlotDesc::Filled(x)) => Ok(x),
            Some(SlotDesc::Placeholder) | None => match self.wait_for_slot_async::<T>(true).await {
                Some(x) => Ok(x),
                None => match T::construct_async(self.as_ref()).await {
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
            None => match T::construct_async(self.as_ref()).await {
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

    /// Builder method equivalent to calling `try_init_async()` but can be chained.
    pub async fn try_with_constructed_async<T: AsyncConstructibleResource>(
        self,
    ) -> Result<Aero<HCons<T, R>>, T::Error> {
        self.try_init_async::<T>().await?;
        Ok(Aero {
            inner: self.inner,
            phantom: PhantomData,
        })
    }

    /// Builder method equivalent to calling `try_init_async()` but can be chained. Panics if construction fails.
    pub async fn with_constructed_async<T: AsyncConstructibleResource>(self) -> Aero<HCons<T, R>> {
        unwrap_constructed::<T, _>(self.try_with_constructed_async().await)
    }

    /// Convert into a different variant of the Aero type. Any missing required resources
    /// will be automatically asynchronously constructed.
    pub async fn try_construct_remaining_async<R2, I>(self) -> anyhow::Result<Aero<R2>>
    where
        R2: Sculptor<R, I> + ResourceList,
        <R2 as Sculptor<R, I>>::Remainder: AsyncConstructibleResourceList,
    {
        <<R2 as Sculptor<R, I>>::Remainder>::construct_async(&self).await?;
        Ok(Aero {
            inner: self.inner,
            phantom: PhantomData,
        })
    }

    /// Convert into a different variant of the Aero type. Any missing required resources
    /// will be automatically asynchronously constructed. Panics if construction of any missing resource fails.
    pub async fn construct_remaining_async<R2, I>(self) -> Aero<R2>
    where
        R2: Sculptor<R, I> + ResourceList,
        <R2 as Sculptor<R, I>>::Remainder: AsyncConstructibleResourceList,
    {
        unwrap_constructed_hlist::<<R2 as Sculptor<R, I>>::Remainder, _>(
            self.try_construct_remaining_async().await,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::{convert::Infallible, time::Duration};

    use crate::Aero;

    use super::*;

    #[derive(Debug, Clone)]
    struct Dummy;

    #[async_trait]
    impl AsyncConstructible for Dummy {
        type Error = Infallible;

        async fn construct_async(_app_state: &Aero) -> Result<Self, Self::Error> {
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok(Self)
        }
    }

    #[tokio::test]
    async fn obtain() {
        let state = Aero::new();
        state.obtain_async::<Dummy>().await;
    }

    #[tokio::test]
    async fn obtain_race() {
        let state = Aero::new();
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

        async fn construct_async(aero: &Aero) -> Result<Self, Self::Error> {
            aero.obtain_async::<Dummy>().await;
            Ok(Self)
        }
    }

    #[tokio::test]
    async fn obtain_recursive() {
        let state = Aero::new();
        state.obtain_async::<DummyRecursive>().await;
    }

    #[tokio::test]
    async fn obtain_recursive_race() {
        let state = Aero::new();
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

        async fn construct_async(aero: &Aero) -> Result<Self, Self::Error> {
            aero.obtain_async::<DummyCyclic>().await;
            Ok(Self)
        }
    }

    #[tokio::test]
    #[should_panic(expected = "Cycle detected")]
    async fn obtain_cyclic() {
        let state = Aero::new();
        state.obtain_async::<DummyCyclic>().await;
    }

    #[derive(Debug, Clone)]
    struct DummySync;

    impl Constructible for DummySync {
        type Error = Infallible;

        fn construct(_app_state: &Aero) -> Result<Self, Self::Error> {
            std::thread::sleep(Duration::from_millis(100));
            Ok(Self)
        }
    }

    #[derive(Debug, Clone)]
    struct DummySyncRecursive;

    #[async_trait]
    impl AsyncConstructible for DummySyncRecursive {
        type Error = Infallible;

        async fn construct_async(aero: &Aero) -> Result<Self, Self::Error> {
            aero.obtain_async::<DummySync>().await;
            Ok(Self)
        }
    }

    #[tokio::test]
    async fn obtain_sync_recursive() {
        let state = Aero::new();
        state.obtain_async::<DummySyncRecursive>().await;
    }

    #[tokio::test]
    async fn obtain_sync_recursive_race() {
        let state = Aero::new();
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

        async fn construct_async(_app_state: &Aero) -> Result<Self, Self::Error> {
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok(Self)
        }
    }

    #[tokio::test]
    async fn obtain_non_clone() {
        let state = Aero::new();
        state.obtain_async::<Arc<DummyNonClone>>().await;
    }

    trait DummyTrait: Send + Sync {}

    #[derive(Debug)]
    struct DummyImpl;

    impl DummyTrait for DummyImpl {}

    #[async_trait]
    impl AsyncConstructible for DummyImpl {
        type Error = Infallible;

        async fn construct_async(_app_state: &Aero) -> Result<Self, Self::Error> {
            Ok(Self)
        }

        async fn after_construction_async(
            this: &(dyn Any + Send + Sync),
            aero: &Aero,
        ) -> Result<(), Self::Error> {
            if let Some(arc) = this.downcast_ref::<Arc<Self>>() {
                aero.insert(arc.clone() as Arc<dyn DummyTrait>)
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn obtain_impl() {
        let state = Aero::new();
        state.init_async::<Arc<DummyImpl>>().await;
        state.try_get_async::<Arc<dyn DummyTrait>>().await.unwrap();
    }

    #[tokio::test]
    async fn with_constructed_async() {
        let state = Aero::new()
            .with(42)
            .with_constructed_async::<Dummy>()
            .await
            .with("hi");
        state.get::<Dummy, _>();
    }

    #[tokio::test]
    async fn construct_remaining_async() {
        let state: Aero![i32, Dummy, DummyRecursive, &str] = Aero::new()
            .with(42)
            .with("hi")
            .construct_remaining_async()
            .await;
        state.get::<Dummy, _>();
        state.get::<DummyRecursive, _>();
    }
}
