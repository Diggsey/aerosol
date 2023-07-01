use std::{task::Poll, thread};

use crate::{
    resource::{unwrap_resource, Resource},
    slot::SlotDesc,
    state::Aerosol,
};

#[cfg(target_family = "wasm")]
pub fn safe_park() {
    panic!("Cannot block on dependency construction on WASM")
}

#[cfg(not(target_family = "wasm"))]
pub fn safe_park() {
    std::thread::park();
}

impl Aerosol {
    /// Synchronously wait for the slot for `T` to not have a placeholder.
    /// Returns immediately if there is no `T` present, or if `T`'s slot is filled.
    pub(crate) fn wait_for_slot<T: Resource>(&self, insert_placeholder: bool) -> Option<T> {
        let mut wait_index = None;
        loop {
            match self.poll_for_slot(&mut wait_index, thread::current, insert_placeholder) {
                Poll::Pending => safe_park(),
                Poll::Ready(x) => break x,
            }
        }
    }

    /// Tries to get an instance of `T` from the AppState. Returns `None` if there is no such instance.
    /// This function does not attempt to construct `T` if it does not exist.
    pub fn try_get<T: Resource>(&self) -> Option<T> {
        match self.try_get_slot()? {
            SlotDesc::Filled(x) => Some(x),
            SlotDesc::Placeholder => self.wait_for_slot::<T>(false),
        }
    }
    /// Get an instance of `T` from the AppState, and panic if not found.
    /// This function does not attempt to construct `T` if it does not exist.
    pub fn get<T: Resource>(&self) -> T {
        unwrap_resource(self.try_get())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_with() {
        let state = Aerosol::new().with(42);
        assert_eq!(state.get::<i32>(), 42);
    }

    #[test]
    fn get_inserted() {
        let state = Aerosol::new();
        state.insert(42);
        assert_eq!(state.get::<i32>(), 42);
    }

    #[test]
    fn try_get_some() {
        let state = Aerosol::new().with(42);
        assert_eq!(state.try_get::<i32>(), Some(42));
    }

    #[test]
    fn try_get_none() {
        let state = Aerosol::new().with("Hello");
        assert_eq!(state.try_get::<i32>(), None);
    }
}
