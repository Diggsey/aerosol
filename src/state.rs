use std::{any::Any, marker::PhantomData, sync::Arc, task::Poll};

use anymap::hashbrown::{Entry, Map};
use frunk::{
    hlist::{HFoldRightable, Sculptor},
    prelude::HList,
    HCons, HNil, Poly,
};
use parking_lot::RwLock;

use crate::{
    resource::{cyclic_resource, duplicate_resource, Resource},
    slot::{Slot, SlotDesc, ThreadOrWaker},
};

#[derive(Debug, Default)]
struct InnerAerosol {
    items: Map<dyn Any + Send + Sync>,
}

/// Stores a collection of resources keyed on resource type.
/// Provides methods for accessing this collection.
/// Can be cheaply cloned.
#[derive(Debug)]
#[repr(transparent)]
pub struct Aerosol<R: HList = HNil> {
    inner: Arc<RwLock<InnerAerosol>>,
    phantom: PhantomData<Arc<R>>,
}

impl Aerosol {
    /// Construct a new instance of the type with no initial resources.
    pub fn new() -> Self {
        Self {
            inner: Default::default(),
            phantom: PhantomData,
        }
    }
}

impl<R: HList> Clone for Aerosol<R> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            phantom: PhantomData,
        }
    }
}

struct AerosolBuilderFolder;
impl<R: HList, T: Resource> frunk::Func<(Aerosol<R>, T)> for AerosolBuilderFolder {
    type Output = Aerosol<HCons<T, R>>;
    fn call((aero, value): (Aerosol<R>, T)) -> Self::Output {
        aero.with(value)
    }
}

#[doc(hidden)]
pub trait HTestable {
    fn test<R: HList>(aero: &Aerosol<R>) -> bool;
}

impl HTestable for HNil {
    fn test<R: HList>(_aero: &Aerosol<R>) -> bool {
        true
    }
}

impl<H: Resource, T: HTestable> HTestable for HCons<H, T> {
    fn test<R: HList>(aero: &Aerosol<R>) -> bool {
        aero.has::<H>() && T::test(aero)
    }
}

impl<
        R: Default + HFoldRightable<Poly<AerosolBuilderFolder>, Aerosol, Output = Aerosol<R>> + HList,
    > Default for Aerosol<R>
{
    fn default() -> Self {
        R::default().foldr(Poly(AerosolBuilderFolder), Aerosol::new())
    }
}

impl<R: HList> Aerosol<R> {
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
    pub fn with<T: Resource>(self, value: T) -> Aerosol<HCons<T, R>> {
        self.insert(value);
        Aerosol {
            inner: self.inner,
            phantom: PhantomData,
        }
    }

    /// Convert into a different variant of the Aerosol type. The new variant must
    /// not require any resources which are not required as part of this type.
    pub fn into<R2: HList, I>(self) -> Aerosol<R2>
    where
        R: Sculptor<R2, I>,
    {
        Aerosol {
            inner: self.inner,
            phantom: PhantomData,
        }
    }

    /// Reborrow as a different variant of the Aerosol type. The new variant must
    /// not require any resources which are not required as part of this type.
    #[allow(clippy::should_implement_trait)]
    pub fn as_ref<R2: HList, I>(&self) -> &Aerosol<R2>
    where
        R: Sculptor<R2, I>,
    {
        // Safety: all Aerosol variants are `#[repr(transparent)]` wrappers around
        // the same concrete type.
        unsafe { std::mem::transmute(self) }
    }

    /// Try to convert into a different variant of the Aerosol type. Returns the
    /// original type if one or more of the required resources are not fully
    /// constructed.
    pub fn try_into<R2: HList + HTestable>(self) -> Result<Aerosol<R2>, Self> {
        if R2::test(&self) {
            Ok(Aerosol {
                inner: self.inner,
                phantom: PhantomData,
            })
        } else {
            Err(self)
        }
    }

    /// Try to convert into a different variant of the Aerosol type. Returns
    /// `None` if one or more of the required resources are not fully
    /// constructed.
    pub fn try_as_ref<R2: HList + HTestable>(&self) -> Option<&Aerosol<R2>> {
        if R2::test(self) {
            Some(
                // Safety: all Aerosol variants are `#[repr(transparent)]` wrappers around
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

impl<R: HList> AsRef<Aerosol> for Aerosol<R> {
    fn as_ref(&self) -> &Aerosol {
        Aerosol::as_ref(self)
    }
}

impl<H, T: HList> From<Aerosol<HCons<H, T>>> for Aerosol {
    fn from(value: Aerosol<HCons<H, T>>) -> Self {
        value.into()
    }
}

#[cfg(test)]
mod tests {
    use crate::Aero;

    use super::*;

    #[test]
    fn create() {
        let state = Aerosol::new().with(42);
        state.insert("Hello, world!");
    }

    #[test]
    #[should_panic]
    fn duplicate() {
        let state = Aerosol::new().with(13);
        state.insert(42);
    }

    #[test]
    fn default() {
        let state: Aero![i32] = Aerosol::default();
        state.insert("Hello, world!");
    }

    #[test]
    fn convert() {
        let state: Aero![i32, String, f32] = Aerosol::default();
        state.insert("Hello, world!");
        let state2: Aero![f32, String] = state.into();
        let _state3: Aero![i32, String, f32] = state2.try_into().unwrap();
    }
}
