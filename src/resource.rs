use std::any::{type_name, Any};

/// Bound on the types that can be used as an aerosol resource.
pub trait Resource: Any + Send + Sync + Clone {}
impl<T: Any + Send + Sync + Clone> Resource for T {}

pub(crate) fn unwrap_resource<T: Resource>(opt: Option<T>) -> T {
    if let Some(value) = opt {
        value
    } else {
        panic!("Resource `{}` does not exist", type_name::<T>())
    }
}

pub(crate) fn unwrap_constructed<T: Resource, U>(res: Result<U, impl Into<anyhow::Error>>) -> U {
    match res {
        Ok(x) => x,
        Err(e) => panic!("Failed to construct `{}`: {}", type_name::<T>(), e.into()),
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
