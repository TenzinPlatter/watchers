use std::{
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

use log::debug;

use crate::git::EventContext;

pub struct Debouncer<F> {
    callback: Arc<Mutex<F>>,
    delay: Duration,
    cancel_signal: Arc<(Mutex<bool>, Condvar)>,
    current_thread: Option<JoinHandle<()>>,
    pending_context: Arc<Mutex<Option<EventContext>>>,
}

impl<F> Debouncer<F>
where
    F: FnMut(EventContext) + Send + 'static,
{
    pub fn new(callback: F, delay: Duration) -> Self {
        Self {
            callback: Arc::new(Mutex::new(callback)),
            delay,
            cancel_signal: Arc::new((Mutex::new(false), Condvar::new())),
            current_thread: None,
            pending_context: Arc::new(Mutex::new(None)),
        }
    }

    pub fn on_event(&mut self, context: EventContext) {
        *self.pending_context.lock().unwrap() = Some(context);
        debug!("got context lock");

        self.cancel_current_thread();
        debug!("cancelled thread");

        let callback = Arc::clone(&self.callback);
        let pending_context = Arc::clone(&self.pending_context);
        let cancel_signal = Arc::new((Mutex::new(false), Condvar::new()));
        self.cancel_signal = Arc::clone(&cancel_signal);
        let delay = self.delay;

        let handle = thread::spawn(move || {
            let (lock, cvar) = &*cancel_signal;
            let mut cancelled = lock.lock().unwrap();

            let result = cvar.wait_timeout(cancelled, delay).unwrap();
            cancelled = result.0;

            if (!*cancelled && result.1.timed_out())
                && let (Ok(mut cb), Ok(mut context_guard)) =
                    (callback.lock(), pending_context.lock())
                && let Some(context) = context_guard.take()
            {
                cb(context);
            }
        });

        self.current_thread = Some(handle);
    }
}

impl<F> Debouncer<F> {
    fn cancel_current_thread(&mut self) {
        if self.current_thread.is_some() {
            let (lock, cvar) = &*self.cancel_signal;
            let mut cancelled = lock.lock().unwrap();
            *cancelled = true;
            cvar.notify_all();

            let _old_handle = self.current_thread.take();
        }
    }
}

impl<F> Drop for Debouncer<F> {
    fn drop(&mut self) {
        self.cancel_current_thread();
    }
}
