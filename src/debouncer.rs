//! Debouncing mechanism for file system events.
//!
//! This module provides a thread-safe debouncer that delays callback execution
//! until after a configurable quiet period. When new events occur before the
//! timer expires, the previous timer is cancelled and a new one is started.
//!
//! The debouncer uses condition variables for efficient thread coordination,
//! allowing for immediate cancellation of pending timers when new events arrive.

use std::{
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

use log::debug;

use crate::git::EventContext;

/// A thread-safe debouncer that delays callback execution.
///
/// The `Debouncer` implements a "last writer wins" strategy where each new event
/// cancels any pending timer and starts a fresh delay period. Only when the delay
/// period completes without interruption is the callback executed.
///
/// # Type Parameters
///
/// * `F` - The callback function type that takes an `EventContext`
///
/// # Example
///
/// ```rust,no_run
/// use watchers::debouncer::Debouncer;
/// use std::time::Duration;
///
/// let debouncer = Debouncer::new(
///     |context| {
///         println!("Processing after quiet period: {:?}", context.repo_path);
///     },
///     Duration::from_secs(2)
/// );
/// ```
///
/// # Thread Safety
///
/// The debouncer is fully thread-safe and can be used from multiple threads
/// simultaneously. Internal synchronization ensures that only the most recent
/// event's callback will execute.
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
    /// Creates a new `Debouncer` with the specified callback and delay.
    ///
    /// # Arguments
    ///
    /// * `callback` - Function to execute after the debounce period
    /// * `delay` - Duration to wait after the last event before executing the callback
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use watchers::debouncer::Debouncer;
    /// use std::time::Duration;
    ///
    /// let debouncer = Debouncer::new(
    ///     |context| println!("Delayed execution!"),
    ///     Duration::from_millis(500)
    /// );
    /// ```
    pub fn new(callback: F, delay: Duration) -> Self {
        Self {
            callback: Arc::new(Mutex::new(callback)),
            delay,
            cancel_signal: Arc::new((Mutex::new(false), Condvar::new())),
            current_thread: None,
            pending_context: Arc::new(Mutex::new(None)),
        }
    }

    /// Triggers the debouncer with a new event.
    ///
    /// If a timer is already running, it will be cancelled and a new timer
    /// will be started with the provided context. The callback will only
    /// execute if the delay period completes without another event occurring.
    ///
    /// # Arguments
    ///
    /// * `context` - Event context containing repository and configuration data
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use watchers::{debouncer::Debouncer, git::EventContext, Config};
    /// use std::{time::Duration, path::PathBuf};
    ///
    /// let mut debouncer = Debouncer::new(
    ///     |ctx| println!("Processing: {:?}", ctx.repo_path),
    ///     Duration::from_secs(1)
    /// );
    ///
    /// let context = EventContext {
    ///     repo_path: PathBuf::from("/tmp/repo"),
    ///     config: Config {
    ///         watch_dir: PathBuf::from("/tmp"),
    ///         commit_delay_secs: 1,
    ///         auto_push: false,
    ///         config_path: None,
    ///     },
    /// };
    ///
    /// debouncer.on_event(context);
    /// ```
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::PathBuf, sync::{Arc, Mutex}, thread, time::Duration};
    use crate::config::Config;

    fn create_test_context() -> EventContext {
        EventContext {
            repo_path: PathBuf::from("/tmp/test"),
            config: Config {
                watch_dir: PathBuf::from("/tmp/test"),
                commit_delay_secs: 1,
                auto_push: false,
                config_path: None,
            },
        }
    }

    #[test]
    fn debouncer_executes_callback_after_delay() {
        let executed = Arc::new(Mutex::new(false));
        let executed_clone = Arc::clone(&executed);

        let mut debouncer = Debouncer::new(
            move |_context| { *executed_clone.lock().unwrap() = true; },
            Duration::from_millis(50)
        );

        debouncer.on_event(create_test_context());
        thread::sleep(Duration::from_millis(100));

        assert!(*executed.lock().unwrap());
    }

    #[test]
    fn debouncer_cancels_previous_timer() {
        let call_count = Arc::new(Mutex::new(0));
        let call_count_clone = Arc::clone(&call_count);

        let mut debouncer = Debouncer::new(
            move |_context| { *call_count_clone.lock().unwrap() += 1; },
            Duration::from_millis(100)
        );

        debouncer.on_event(create_test_context());
        thread::sleep(Duration::from_millis(50));
        debouncer.on_event(create_test_context()); // Should cancel first
        thread::sleep(Duration::from_millis(150));

        assert_eq!(*call_count.lock().unwrap(), 1); // Only second callback executes
    }

    #[test]
    fn debouncer_handles_rapid_events() {
        let call_count = Arc::new(Mutex::new(0));
        let call_count_clone = Arc::clone(&call_count);

        let mut debouncer = Debouncer::new(
            move |_context| { *call_count_clone.lock().unwrap() += 1; },
            Duration::from_millis(50)
        );

        // Trigger many rapid events
        for _ in 0..10 {
            debouncer.on_event(create_test_context());
            thread::sleep(Duration::from_millis(5));
        }

        thread::sleep(Duration::from_millis(100));

        assert_eq!(*call_count.lock().unwrap(), 1); // Only one callback should execute
    }
}