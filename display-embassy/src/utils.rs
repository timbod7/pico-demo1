use core::cell::RefCell;
use core::ops::Deref;

use embassy_sync::{
    blocking_mutex::{raw::CriticalSectionRawMutex, ThreadModeMutex},
    signal::Signal,
};

/// Combines a state value and a message describing updates to it
pub struct StateAndSignal<S, M> {
    pub state: ThreadModeMutex<RefCell<S>>,
    pub signal: Signal<CriticalSectionRawMutex, M>,
}

impl<S, M> StateAndSignal<S, M>
where
    M: Send,
{
    pub const fn new(init: S) -> Self {
        let state = ThreadModeMutex::new(RefCell::new(init));
        let signal = Signal::new();
        StateAndSignal { state, signal }
    }

    pub fn update<F>(&self, mut ufn: F)
    where
        F: FnMut(&mut S) -> M,
    {
        let m = self.state.lock(|s| ufn(&mut s.borrow_mut()));
        self.signal.signal(m);
    }

    pub async fn wait<F, T>(&self, mut hfn: F) -> T
    where
        F: FnMut(&M, &S) -> T,
    {
        let m = self.signal.wait().await;
        self.state.lock(|s| hfn(&m, s.borrow().deref()))
    }
}
