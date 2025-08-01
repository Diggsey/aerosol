//! Integration with the `axum` web framework.
//!
//! Provides the `Dep` and `Obtain` axum extractors for easily accessing
//! resources from within route handlers.
//!
//! To make use of these extractors, your application state must either be
//! an `Aero`, or you must implement `FromRef<YourState>` for `Aero`.

use std::any::type_name;
use std::convert::Infallible;

use axum::{
    extract::{FromRef, FromRequestParts, OptionalFromRequestParts},
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use frunk::HCons;

use crate::{Aero, AsyncConstructibleResource, Resource, ResourceList};

#[cfg(feature = "aide")]
mod aide;

/// Type of axum Rejection returned when a resource cannot be acquired
#[derive(Debug, thiserror::Error)]
pub enum DependencyError {
    /// Tried to get a resource which did not exist. Use `Obtain(..)` if you want aerosol to
    /// try to construct the resource on demand.
    #[error("Resource `{name}` does not exist")]
    DoesNotExist {
        /// Name of the resource type
        name: &'static str,
    },
    /// Tried and failed to construct a resource.
    #[error("Failed to construct `{name}`: {source}")]
    FailedToConstruct {
        /// Name of the resource type
        name: &'static str,
        /// Error returned by the resource constructor
        #[source]
        source: anyhow::Error,
    },
}

impl IntoResponse for DependencyError {
    fn into_response(self) -> Response {
        tracing::error!("{}", self);
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

impl DependencyError {
    pub(crate) fn does_not_exist<T>() -> Self {
        Self::DoesNotExist {
            name: type_name::<T>(),
        }
    }
    pub(crate) fn failed_to_construct<T>(error: impl Into<anyhow::Error>) -> Self {
        Self::FailedToConstruct {
            name: type_name::<T>(),
            source: error.into(),
        }
    }
}

/// Get an already-existing resource from the state. Equivalent to calling `Aero::try_get_async`.
pub struct Dep<T: Resource>(pub T);

impl<T: Resource, S: Send + Sync> FromRequestParts<S> for Dep<T>
where
    Aero: FromRef<S>,
{
    type Rejection = DependencyError;

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Aero::from_ref(state)
            .try_get_async()
            .await
            .map(Self)
            .ok_or_else(DependencyError::does_not_exist::<T>)
    }
}

impl<T: Resource, S: Send + Sync> OptionalFromRequestParts<S> for Dep<T>
where
    Aero: FromRef<S>,
{
    type Rejection = Infallible;

    // Required method
    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        Ok(
            <Self as FromRequestParts<S>>::from_request_parts(parts, state)
                .await
                .ok(),
        )
    }
}

/// Get a resource from the state, or construct it if it doesn't exist. Equivalent to calling `Aero::try_obtain_async`.
pub struct Obtain<T: AsyncConstructibleResource>(pub T);

impl<T: AsyncConstructibleResource, S: Send + Sync> FromRequestParts<S> for Obtain<T>
where
    Aero: FromRef<S>,
{
    type Rejection = DependencyError;

    async fn from_request_parts(_parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        Aero::from_ref(state)
            .try_obtain_async()
            .await
            .map(Self)
            .map_err(DependencyError::failed_to_construct::<T>)
    }
}

impl<T: AsyncConstructibleResource, S: Send + Sync> OptionalFromRequestParts<S> for Obtain<T>
where
    Aero: FromRef<S>,
{
    type Rejection = Infallible;

    // Required method
    async fn from_request_parts(
        parts: &mut Parts,
        state: &S,
    ) -> Result<Option<Self>, Self::Rejection> {
        Ok(
            <Self as FromRequestParts<S>>::from_request_parts(parts, state)
                .await
                .ok(),
        )
    }
}

impl<H: Resource, T: ResourceList> FromRef<Aero<HCons<H, T>>> for Aero {
    fn from_ref(input: &Aero<HCons<H, T>>) -> Self {
        input.clone().into()
    }
}
#[cfg(feature = "axum-extra")]
mod extra_impls {
    use axum::extract::FromRef;

    use crate::{Aero, ResourceList};

    impl<R: ResourceList> FromRef<Aero<R>> for axum_extra::extract::cookie::Key {
        fn from_ref(input: &Aero<R>) -> Self {
            input.try_get().expect("Missing cookie key")
        }
    }
}
