use std::{any::Any, sync::Arc, task::Poll};

use anymap::hashbrown::{Entry, Map};
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
/// Provies methods for accessing this collection.
/// Can be cheaply cloned.
#[derive(Debug, Clone, Default)]
pub struct Aerosol {
    inner: Arc<RwLock<InnerAerosol>>,
}

impl Aerosol {
    /// Construct a new instance of the type with no initial resources.
    pub fn new() -> Self {
        Self::default()
    }
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
    pub fn with<T: Resource>(self, value: T) -> Self {
        self.insert(value);
        self
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

#[cfg(test)]
mod tests {
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
}
