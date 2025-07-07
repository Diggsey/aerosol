use aide::OperationInput;

use crate::{
    axum::{Dep, Obtain},
    AsyncConstructibleResource, Resource,
};

impl<T: Resource> OperationInput for Dep<T> {}
impl<T: AsyncConstructibleResource> OperationInput for Obtain<T> {}
