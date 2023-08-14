use std::{any::Any, fmt::Debug, marker::PhantomData, sync::Arc, task::Poll};

use anymap::hashbrown::{Entry, Map};
use frunk::{
    hlist::{HFoldRightable, Sculptor},
    HCons, HNil, Poly,
};
use parking_lot::RwLock;

use crate::{
    resource::{cyclic_resource, duplicate_resource, missing_resource, Resource, ResourceList},
    slot::{Slot, SlotDesc, ThreadOrWaker},
};

#[derive(Debug, Default)]
pub(crate) struct InnerAero {
    items: Map<dyn Any + Send + Sync>,
}

/// Stores a collection of resources keyed on resource type.
/// Provides methods for accessing this collection.
/// Can be cheaply cloned.
#[repr(transparent)]
pub struct Aero<R: ResourceList = HNil> {
    pub(crate) inner: Arc<RwLock<InnerAero>>,
    pub(crate) phantom: PhantomData<Arc<R>>,
}

impl<R: ResourceList> Debug for Aero<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl Aero {
    /// Construct a new instance of the type with no initial resources.
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
            phantom: PhantomData,
        }
    }
}

impl<R: ResourceList> Clone for Aero<R> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            phantom: PhantomData,
        }
    }
}

struct AerosolDefaultFolder;
impl<R: ResourceList, T: Resource> frunk::Func<(Aero<R>, T)> for AerosolDefaultFolder {
    type Output = Aero<HCons<T, R>>;
    fn call((aero, value): (Aero<R>, T)) -> Self::Output {
        aero.with(value)
    }
}

impl<
        R: Default + HFoldRightable<Poly<AerosolDefaultFolder>, Aero, Output = Aero<R>> + ResourceList,
    > Default for Aero<R>
{
    fn default() -> Self {
        R::default().foldr(Poly(AerosolDefaultFolder), Aero::new())
    }
}

impl<R: ResourceList> Aero<R> {
    /// Directly insert a resource into the collection. Panics if a resource of the
    /// same type already exists.
    pub fn insert<T: Resource>(&self, value: T) {
        match self.inner.write().items.entry() {
            Entry::Occupied(_) => duplicate_resource::<T>(),
            Entry::Vacant(vac) => {
                vac.insert(Slot::Filled(value));
            }
        }
    }

    /// Builder method equivalent to calling `insert()` but can be chained.
    pub fn with<T: Resource>(self, value: T) -> Aero<HCons<T, R>> {
        self.insert(value);
        Aero {
            inner: self.inner,
            phantom: PhantomData,
        }
    }

    /// Convert into a different variant of the Aero type. The new variant must
    /// not require any resources which are not required as part of this type.
    pub fn into<R2: ResourceList, I>(self) -> Aero<R2>
    where
        R: Sculptor<R2, I>,
    {
        Aero {
            inner: self.inner,
            phantom: PhantomData,
        }
    }

    /// Reborrow as a different variant of the Aero type. The new variant must
    /// not require any resources which are not required as part of this type.
    #[allow(clippy::should_implement_trait)]
    pub fn as_ref<R2: ResourceList, I>(&self) -> &Aero<R2>
    where
        R: Sculptor<R2, I>,
    {
        // Safety: all Aero variants are `#[repr(transparent)]` wrappers around
        // the same concrete type.
        unsafe { std::mem::transmute(self) }
    }

    /// Try to convert into a different variant of the Aero type. Returns the
    /// original type if one or more of the required resources are not fully
    /// constructed.
    pub fn try_into<R2: ResourceList>(self) -> Result<Aero<R2>, Self> {
        if R2::test(&self) {
            Ok(Aero {
                inner: self.inner,
                phantom: PhantomData,
            })
        } else {
            Err(self)
        }
    }

    /// Try to convert into a different variant of the Aero type. Returns
    /// `None` if one or more of the required resources are not fully
    /// constructed.
    pub fn try_as_ref<R2: ResourceList>(&self) -> Option<&Aero<R2>> {
        if R2::test(self) {
            Some(
                // Safety: all Aero variants are `#[repr(transparent)]` wrappers around
                // the same concrete type.
                unsafe { std::mem::transmute(self) },
            )
        } else {
            None
        }
    }

    /// Check if a resource with a specific type is fully constructed in this
    /// aerosol instance
    pub fn has<T: Resource>(&self) -> bool {
        matches!(
            self.inner.read().items.get::<Slot<T>>(),
            Some(Slot::Filled(_))
        )
    }

    /// Assert that a resource exists, returns `self` unchanged if not
    pub fn try_assert<T: Resource>(self) -> Result<Aero<HCons<T, R>>, Self> {
        if self.has::<T>() {
            Ok(Aero {
                inner: self.inner,
                phantom: PhantomData,
            })
        } else {
            Err(self)
        }
    }

    /// Assert that a resource exists, panic if not
    pub fn assert<T: Resource>(self) -> Aero<HCons<T, R>> {
        self.try_assert()
            .unwrap_or_else(|_| missing_resource::<T>())
    }

    pub(crate) fn try_get_slot<T: Resource>(&self) -> Option<SlotDesc<T>> {
        self.inner.read().items.get().map(Slot::desc)
    }
    pub(crate) fn poll_for_slot<T: Resource, C: Into<ThreadOrWaker>>(
        &self,
        wait_index: &mut Option<usize>,
        thread_or_waker_fn: impl Fn() -> C,
        insert_placeholder: bool,
    ) -> Poll<Option<T>> {
        let mut guard = self.inner.write();
        match guard.items.entry::<Slot<T>>() {
            Entry::Occupied(mut occ) => match occ.get_mut() {
                Slot::Filled(x) => Poll::Ready(Some(x.clone())),
                Slot::Placeholder { owner, waiting } => {
                    let current = thread_or_waker_fn().into();
                    if current == *owner {
                        cyclic_resource::<T>()
                    }
                    if let Some(idx) = *wait_index {
                        waiting[idx] = current;
                    } else {
                        *wait_index = Some(waiting.len());
                        waiting.push(current);
                    }
                    Poll::Pending
                }
            },
            Entry::Vacant(vac) => {
                if insert_placeholder {
                    vac.insert(Slot::Placeholder {
                        owner: thread_or_waker_fn().into(),
                        waiting: Vec::new(),
                    });
                }
                Poll::Ready(None)
            }
        }
    }

    pub(crate) fn fill_placeholder<T: Resource>(&self, value: T) {
        self.inner.write().items.insert(Slot::Filled(value));
    }
    pub(crate) fn clear_placeholder<T: Resource>(&self) {
        self.inner.write().items.remove::<Slot<T>>();
    }
}

impl<R: ResourceList> AsRef<Aero> for Aero<R> {
    fn as_ref(&self) -> &Aero {
        Aero::as_ref(self)
    }
}

impl<H: Resource, T: ResourceList> From<Aero<HCons<H, T>>> for Aero {
    fn from(value: Aero<HCons<H, T>>) -> Self {
        value.into()
    }
}

#[cfg(test)]
mod tests {
    use crate::Aero;

    #[test]
    fn create() {
        let state = Aero::new().with(42);
        state.insert("Hello, world!");
    }

    #[test]
    #[should_panic]
    fn duplicate() {
        let state = Aero::new().with(13);
        state.insert(42);
    }

    #[test]
    fn default() {
        let state: Aero![i32] = Aero::default();
        state.insert("Hello, world!");
    }

    #[test]
    fn convert() {
        let state: Aero![i32, String, f32] = Aero::default();
        state.insert("Hello, world!");
        let state2: Aero![f32, String] = state.into();
        let _state3: Aero![i32, String, f32] = state2.try_into().unwrap();
    }

    #[test]
    fn assert() {
        let state: Aero![i32, String, f32] = Aero::default();
        state.insert("Hello, world!");
        let _state2: Aero![&str, f32] = state.assert::<&str>().into();
    }
}
