use std::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{
    resource::{Resource, ResourceList},
    slot::SlotDesc,
    state::Aero,
};

pub(crate) struct WaitForSlot<R: ResourceList, T: Resource> {
    state: Aero<R>,
    wait_index: Option<usize>,
    insert_placeholder: bool,
    phantom: PhantomData<fn() -> T>,
}

impl<R: ResourceList, T: Resource> Future for WaitForSlot<R, T> {
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        this.state
            .poll_for_slot(&mut this.wait_index, || cx.waker(), this.insert_placeholder)
    }
}

impl<R: ResourceList> Aero<R> {
    pub(crate) fn wait_for_slot_async<T: Resource>(
        &self,
        insert_placeholder: bool,
    ) -> WaitForSlot<R, T> {
        WaitForSlot {
            state: self.clone(),
            wait_index: None,
            insert_placeholder,
            phantom: PhantomData,
        }
    }
    /// Tries to get an instance of `T` from the AppState. Returns `None` if there is no such instance.
    /// This function does not attempt to construct `T` if it does not exist.
    pub async fn try_get_async<T: Resource>(&self) -> Option<T> {
        match self.try_get_slot()? {
            SlotDesc::Filled(x) => Some(x),
            SlotDesc::Placeholder => self.wait_for_slot_async::<T>(false).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn try_get_some() {
        let state = Aero::new().with(42);
        assert_eq!(state.try_get_async::<i32>().await, Some(42));
    }

    #[tokio::test]
    async fn try_get_none() {
        let state = Aero::new().with("Hello");
        assert_eq!(state.try_get_async::<i32>().await, None);
    }
}
