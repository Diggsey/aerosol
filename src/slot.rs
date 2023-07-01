#[cfg(feature = "async")]
use std::task::Waker;
use std::thread::Thread;

use crate::resource::Resource;

#[derive(Debug, Clone)]
pub enum ThreadOrWaker {
    Thread(Thread),
    #[cfg(feature = "async")]
    Waker(Waker),
}

impl PartialEq for ThreadOrWaker {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Thread(l0), Self::Thread(r0)) => l0.id() == r0.id(),
            #[cfg(feature = "async")]
            (Self::Waker(l0), Self::Waker(r0)) => l0.will_wake(r0),
            #[cfg(feature = "async")]
            _ => false,
        }
    }
}

impl From<Thread> for ThreadOrWaker {
    fn from(value: Thread) -> Self {
        Self::Thread(value)
    }
}

#[cfg(feature = "async")]
impl From<&Waker> for ThreadOrWaker {
    fn from(value: &Waker) -> Self {
        Self::Waker(value.clone())
    }
}

impl ThreadOrWaker {
    pub fn unpark_or_wake(self) {
        match self {
            ThreadOrWaker::Thread(thread) => thread.unpark(),
            #[cfg(feature = "async")]
            ThreadOrWaker::Waker(waker) => waker.wake(),
        }
    }
}

pub enum Slot<T: Resource> {
    Filled(T),
    Placeholder {
        owner: ThreadOrWaker,
        waiting: Vec<ThreadOrWaker>,
    },
}

impl<T: Resource> Slot<T> {
    pub fn desc(&self) -> SlotDesc<T> {
        if let Slot::Filled(x) = self {
            SlotDesc::Filled(x.clone())
        } else {
            SlotDesc::Placeholder
        }
    }
}

impl<T: Resource> Drop for Slot<T> {
    fn drop(&mut self) {
        if let Self::Placeholder { waiting, .. } = self {
            for item in waiting.drain(..) {
                item.unpark_or_wake();
            }
        }
    }
}

pub enum SlotDesc<T: Resource> {
    Filled(T),
    Placeholder,
}
