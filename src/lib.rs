use std::cell::UnsafeCell;
use std::sync::Once;

/// `ReplaceOnce` provides a synchronization primitive similar to a pre-initialized `OnceCell`,
/// allowing a value to be replaced *at most once* safely across threads.
///
/// Unlike `OnceCell`, the value is initialized at construction and can be atomically
/// replaced exactly once. This is useful for rare cases where a default or placeholder value
/// must be present early, but may be updated later during runtime.
///
/// ## Example
/// ```
/// use replace_once::*;
///
/// let cell = ReplaceOnce::new(42);
/// assert_eq!(*cell.get(), 42);
///
/// let replaced = cell.replace(100);
/// assert_eq!(replaced, ReplaceResult::Replaced(42));
/// assert_eq!(*cell.get(), 100);
///
/// let skipped = cell.replace(200);
/// assert_eq!(skipped, ReplaceResult::Skipped(200));
/// assert_eq!(*cell.get(), 100);
/// ```
///
/// ## Safety
/// `ReplaceOnce` ensures that the replacement operation occurs at most once using
/// `std::sync::Once`. However, the caller must uphold the following invariants:
///
/// - It is **undefined behavior** to perform a replacement concurrently with any access
///   to the wrapped value (`get`, `replace`, etc.).
/// - It is **undefined behavior** to access the value via `get()` while any mutable
///   reference to it is alive.
/// - Synchronization for safe concurrent access is the responsibility of the user.

pub struct ReplaceOnce<T> {
    once: Once,
    value: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for ReplaceOnce<T> {}
unsafe impl<T: Sync> Sync for ReplaceOnce<T> {}

#[derive(Debug)]
pub enum ReplaceResult<T> {
    Skipped(T),
    Replaced(T),
}

impl<T: PartialEq> PartialEq for ReplaceResult<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Skipped(l0), Self::Skipped(r0)) => l0 == r0,
            (Self::Replaced(l0), Self::Replaced(r0)) => l0 == r0,
            _ => false,
        }
    }
}

impl<T> From<ReplaceResult<T>> for Result<T, T> {
    fn from(value: ReplaceResult<T>) -> Self {
        match value {
            ReplaceResult::Skipped(t) => Result::Err(t),
            ReplaceResult::Replaced(t) => Result::Ok(t),
        }
    }
}

impl<T> ReplaceOnce<T> {
    pub const fn new(value: T) -> Self {
        Self {
            once: Once::new(),
            value: UnsafeCell::new(value),
        }
    }

    /// Attempts to replace the current value. If the value has not yet been replaced,
    /// it is replaced with `new`, and the previous value is returned.
    ///
    /// If the value has already been replaced, `new` is returned unchanged.
    ///
    /// # Returns
    /// - `ReplaceResult::Replaced(previous)` if the replacement occurred.
    /// - `ReplaceResult::Skipped(new)` if the replacement was skipped.
    #[must_use]
    pub fn replace(&self, new: T) -> ReplaceResult<T> {
        let mut existing: Option<T> = Option::None;
        let mut new = Option::Some(new);

        self.once.call_once(|| {
            existing.replace(unsafe { self.replace_unchecked(new.take().unwrap()) });
        });

        match existing {
            Some(old) => ReplaceResult::Replaced(old),
            None => ReplaceResult::Skipped(new.unwrap()),
        }
    }

    /// Attempts to replace the current value using the result of a closure `f`.
    ///
    /// If the value has not yet been replaced, the closure is executed and its result is used.
    /// Otherwise, the closure is never invoked.
    ///
    /// # Returns
    /// - `Some(previous)` if replacement occurred.
    /// - `None` if the value was already replaced.
    #[must_use]
    pub fn replace_with<F: FnOnce() -> T>(&self, f: F) -> Option<T> {
        let mut maybe = Option::None;
        self.once.call_once(|| {
            maybe.replace(unsafe { self.replace_unchecked(f()) });
        });
        maybe
    }

    /// Returns `true` if the value has already been replaced.
    #[inline]
    pub fn has_been_replaced(&self) -> bool {
        self.once.is_completed()
    }

    /// Replaces the inner value unconditionally without checking.
    ///
    /// # Safety
    /// This method assumes no other references to the value exist and that no races occur.
    /// It is only called internally within a `Once` guard.
    #[must_use]
    unsafe fn replace_unchecked(&self, value: T) -> T {
        let dst = self.value.get();
        // the value is dropped immediately on update
        unsafe { core::ptr::replace(dst, value) }
    }

    /// Returns a shared reference to the current value.
    ///
    /// # Safety
    /// This must not be called while a mutable reference exists or while a concurrent
    /// replacement operation is in progress. It is the caller’s responsibility to ensure
    /// safe access according to the documented safety contract.
    #[inline]
    pub fn get(&self) -> &T {
        unsafe { &*self.value.get() }
    }

    /// Returns a reference to the current value, and optionally replaces it using a closure `f`.
    ///
    /// The closure is only executed if the value has not yet been replaced.
    ///
    /// # Returns
    /// - A reference to the current value (either the original or replacement).
    /// - `Some(previous)` if a replacement occurred.
    /// - `None` if the closure was skipped.
    #[must_use]
    pub fn get_or_replace_with<F>(&self, f: F) -> (&T, Option<T>)
    where
        F: FnOnce() -> T,
    {
        let mut existing = Option::None;
        self.once.call_once(|| {
            existing.replace(unsafe { self.replace_unchecked(f()) });
        });

        (self.get(), existing)
    }

    /// Returns a reference to the current value, and optionally replaces it with `t`.
    ///
    /// # Returns
    /// - A reference to the current value (either the original or replacement).
    /// - `ReplaceResult::Replaced(previous)` if a replacement occurred.
    /// - `ReplaceResult::Skipped(t)` if the replacement was skipped and `t` is returned.
    #[must_use]
    pub fn get_or_replace(&self, t: T) -> (&T, ReplaceResult<T>) {
        let mut value = Option::from(t);
        let (ref_, maybe_replaced) = self.get_or_replace_with(|| value.take().unwrap());

        match maybe_replaced {
            Some(existing) => (ref_, ReplaceResult::Replaced(existing)),
            None => (ref_, ReplaceResult::Skipped(value.take().unwrap())),
        }
    }

}

impl<T> From<T> for ReplaceOnce<T> {
    fn from(value: T) -> Self {
        ReplaceOnce::new(value)
    }
}

impl<T: Default> Default for ReplaceOnce<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for ReplaceOnce<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReplaceOnce")
            .field("value", self.get())
            .field("replaced", &self.has_been_replaced())
            .finish()
    }
}

#[cfg(test)]
pub mod test_update_once {
    use std::sync::{Arc, Barrier, OnceLock};
    use std::thread::{self, sleep};
    use std::time::Duration;

    use super::ReplaceResult;

    use super::ReplaceOnce;

    #[test]
    fn test_set_methods_update_values_and_return_old_ones() {
        {
            let holder = ReplaceOnce::from(true);
            assert_eq!(*holder.get(), true);
            let existing = holder.replace(false);
            assert_eq!(
                existing,
                ReplaceResult::Replaced(true),
                "Expected to see the existing value (true) wrapped in Result::Ok"
            );
        }

        {
            let holder = ReplaceOnce::from(true);
            assert_eq!(*holder.get(), true);

            let existing = holder.replace_with(|| false);
            assert_eq!(
                existing,
                Some(true),
                "Expected to see the existing value (true) wrapped in Option::Some"
            );
        }
    }

    #[test]
    fn test_replacing_values_just_once() {
        {
            let holder = ReplaceOnce::from(true);
            assert_eq!(*holder.get(), true);
            let _ = holder.replace(false);
            assert!(holder.has_been_replaced());

            let new = holder.replace(true);
            assert_eq!(
                new,
                ReplaceResult::Skipped(true),
                "Expected to see the passed value wrapped in Result::Err since we didn't perform an update"
            );
            assert!(holder.has_been_replaced());
            let new = holder.replace(true);
            assert_eq!(
                new,
                ReplaceResult::Skipped(true),
                "Expected to see the passed value wrapped in Result::Err since we didn't perform an update"
            );
            assert!(holder.has_been_replaced());
            let new = holder.replace(true);
            assert_eq!(
                new,
                ReplaceResult::Skipped(true),
                "Expected to see the passed value wrapped in Result::Err since we didn't perform an update"
            );
            assert!(holder.has_been_replaced());
        }

        {
            let holder = ReplaceOnce::from(true);
            assert_eq!(*holder.get(), true);
            let _ = holder.replace_with(|| false);

            let new = holder.replace_with(|| true);
            assert_eq!(new, None, "Expected to see no updates performed");
            assert!(holder.has_been_replaced());
            let new = holder.replace_with(|| true);
            assert_eq!(new, None, "Expected to see no updates performed");
            assert!(holder.has_been_replaced());
            let new = holder.replace_with(|| true);
            assert_eq!(new, None, "Expected to see no updates performed");
            assert!(holder.has_been_replaced());
        }

        // interleave the two methods
        {
            let holder = ReplaceOnce::from(true);
            assert_eq!(*holder.get(), true);
            let _ = holder.replace(false);
            assert!(holder.has_been_replaced());

            let new = holder.replace_with(|| true);
            assert_eq!(new, None, "Expected to see no updates performed");
            assert!(holder.has_been_replaced());
            let new = holder.replace_with(|| true);
            assert_eq!(new, None, "Expected to see no updates performed");
            assert!(holder.has_been_replaced());
            let new = holder.replace_with(|| true);
            assert_eq!(new, None, "Expected to see no updates performed");
            assert!(holder.has_been_replaced());
        }

        {
            let holder = ReplaceOnce::from(true);
            assert_eq!(*holder.get(), true);
            let _ = holder.replace_with(|| false);

            let new = holder.replace(true);
            assert_eq!(
                new,
                ReplaceResult::Skipped(true),
                "Expected to see the passed value wrapped in Result::Err since we didn't perform an update"
            );
            assert!(holder.has_been_replaced());
            let new = holder.replace(true);
            assert_eq!(
                new,
                ReplaceResult::Skipped(true),
                "Expected to see the passed value wrapped in Result::Err since we didn't perform an update"
            );
            assert!(holder.has_been_replaced());
            let new = holder.replace(true);
            assert_eq!(
                new,
                ReplaceResult::Skipped(true),
                "Expected to see the passed value wrapped in Result::Err since we didn't perform an update"
            );
            assert!(holder.has_been_replaced());
        }
    }

    #[test]
    fn test_get_or_update_methods() {
        {
            let holder = ReplaceOnce::from(true);
            assert_eq!(*holder.get(), true);
            let (new_value_ref, replace_result) = holder.get_or_replace(false);
            assert!(holder.has_been_replaced());
            assert_eq!(
                new_value_ref, &false,
                "Expected a reference to the new value, which is false."
            );
            assert_eq!(
                replace_result,
                ReplaceResult::Replaced(true),
                "Expected to see the previous value wrapped in an Option::Some"
            );

            // verify that the second call still returns a reference to the correct value, and returns a Result::Err that owns the input.
            let (new_value_ref, replace_result) = holder.get_or_replace(true);
            assert_eq!(
                new_value_ref, &false,
                "Expected a reference to the exsting value, which is false"
            );
            assert_eq!(
                replace_result,
                ReplaceResult::Skipped(true),
                "Expected to see the passed value wrapped in Result::Err since we didn't perform an update"
            );
            assert!(holder.has_been_replaced());
        }

        {
            let holder = ReplaceOnce::from(true);
            assert_eq!(*holder.get(), true);

            let new_value = false;
            let (new_value_ref, replace_result) = holder.get_or_replace_with(|| new_value);
            assert_eq!(
                new_value_ref, &false,
                "Expected a reference to the new value, which is false."
            );
            assert_eq!(
                replace_result,
                Option::Some(true),
                "Expected to see the previous value returned wrapped in an Option::Some"
            );
            assert!(holder.has_been_replaced());
            // Seeing the new value is enough to ensure we've invoked the function

            // verify that the second call still returns a reference to the correct value, and returns None since the function was never invoked.
            let mut flag = true;
            let (new_value_ref, replace_result) = holder.get_or_replace_with(|| {
                flag = false;
                false
            });
            assert_eq!(
                new_value_ref, &false,
                "Expected a reference to the exsting value, which is false"
            );
            assert_eq!(
                replace_result,
                Option::None,
                "Expected to see no updates performed"
            );
            assert!(
                flag,
                "Expected flag to be replace with true and thus the function to not have been invoked"
            );
            assert!(holder.has_been_replaced());
        }
    }

    #[test]
    fn test_get_or_replace_with_updates_once_multithread_writes() {
        let initial_value = vec![];
        for _ in 0..10 {
            let ntasks = 200;
            let holder = Arc::new(ReplaceOnce::new(initial_value.clone()));

            // we are going to use the oncelock to hold the new data because it can be shared across threads
            // and has interior mutability.
            let init_value = Arc::new(OnceLock::new());

            // we are using a barrier to create a thundering herd of threads and ensure one, at random picks,
            // picks get entrance;
            let write_barrier = Arc::new(Barrier::new(ntasks + 1));
            let read_barrier = Arc::new(Barrier::new(ntasks + 1));

            {
                let mut handles = vec![];
                for task in 0..ntasks {
                    let value = vec![task];
                    let init_value = init_value.clone();
                    let holder = holder.clone();

                    let write_barrier = write_barrier.clone();
                    let read_barrier = read_barrier.clone();

                    let initial_value = initial_value.clone();

                    // we are going to wait for all threads to sync
                    // s.t. the scheduler decides randomly which one to pick for us
                    // we will try to update the contents from all threads
                    // if update is success, we verify that we picked the initial value
                    // otherwise that we saw None
                    // we then sync the reads with a barrier once more as read while a
                    // write is active is UB.
                    // we then verify that the content of the OnceLock is the same as that
                    // of the ReplaceOnce lock.
                    handles.push(thread::spawn(move || {
                        write_barrier.wait();
                        let mut flag = false;
                        let result = holder.replace_with(|| {
                            let _replaced = init_value.set(value.clone());
                            assert!(matches!((), _replaced));
                            flag = true;
                            value.clone()
                        });

                        match flag {
                            true => {
                                assert_eq!(result, Option::Some(initial_value));
                                assert!(holder.has_been_replaced());
                            }
                            false => assert_eq!(result, None),
                        };

                        read_barrier.wait();
                        assert!(holder.has_been_replaced());
                        assert_eq!(init_value.get().unwrap(), holder.get());
                    }));
                }
                sleep(Duration::from_millis(50));
                write_barrier.wait();
                read_barrier.wait();
                handles.into_iter().for_each(|handle| {
                    handle.join().unwrap();
                });
            };

            assert_eq!(init_value.get().unwrap(), holder.get())
        }
    }
}
