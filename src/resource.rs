use std::any::{type_name, Any};

use frunk::{prelude::HList, HCons, HNil};

use crate::Aero;

/// Bound on the types that can be used as an aerosol resource.
pub trait Resource: Any + Send + Sync + Clone {}
impl<T: Any + Send + Sync + Clone> Resource for T {}

/// A compile-time list of resource types which are statically guaranteed to be present.
pub trait ResourceList: HList + Any + Send + Sync + Clone {
    /// Test at runtmie whether every resource in this list is present in the given Aero instance.
    fn test<R: ResourceList>(aero: &Aero<R>) -> bool;
}
impl ResourceList for HNil {
    fn test<R: ResourceList>(_aero: &Aero<R>) -> bool {
        true
    }
}
impl<H: Resource, T: ResourceList> ResourceList for HCons<H, T> {
    fn test<R: ResourceList>(aero: &Aero<R>) -> bool {
        aero.has::<H>() && T::test(aero)
    }
}

pub(crate) fn missing_resource<T: Resource>() -> ! {
    panic!("Resource `{}` does not exist", type_name::<T>())
}

pub(crate) fn unwrap_resource<T: Resource>(opt: Option<T>) -> T {
    if let Some(value) = opt {
        value
    } else {
        missing_resource::<T>()
    }
}

pub(crate) fn unwrap_constructed<T: Resource, U>(res: Result<U, impl Into<anyhow::Error>>) -> U {
    match res {
        Ok(x) => x,
        Err(e) => panic!("Failed to construct `{}`: {}", type_name::<T>(), e.into()),
    }
}

pub(crate) fn unwrap_constructed_hlist<T, U>(res: Result<U, impl Into<anyhow::Error>>) -> U {
    match res {
        Ok(x) => x,
        Err(e) => panic!(
            "Failed to construct one of `{}`: {}",
            type_name::<T>(),
            e.into()
        ),
    }
}

pub(crate) fn duplicate_resource<T: Resource>() -> ! {
    panic!(
        "Duplicate resource: attempted to add a second `{}`",
        type_name::<T>()
    )
}

pub(crate) fn cyclic_resource<T: Resource>() -> ! {
    panic!(
        "Cycle detected when constructing resource `{}`",
        type_name::<T>()
    )
}
