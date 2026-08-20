//! Declarative View and State Store Abstractions with Time-Travel Debugging.

use crate::reactive::Signal;
use crate::widget::Widget;

/// A declarative component that can be rendered into a virtual UI tree.
pub trait View {
    fn render(&self) -> Widget;
}

impl<F: Fn() -> Widget> View for F {
    fn render(&self) -> Widget {
        (self)()
    }
}

/// An application-level state store with history recording and time-travel debugging.
pub struct StateStore<T: Clone + 'static> {
    current: Signal<T>,
    history: Vec<T>,
    history_index: usize,
}

impl<T: Clone + 'static> StateStore<T> {
    pub fn new(initial: T) -> Self {
        Self {
            current: Signal::new(initial.clone()),
            history: vec![initial],
            history_index: 0,
        }
    }

    /// Read the current state value reactively.
    pub fn get(&self) -> T {
        self.current.get()
    }

    /// Dispatch a state transformation, recording a time-travel history checkpoint.
    pub fn dispatch(&mut self, reducer: impl FnOnce(&T) -> T) {
        let next_state = reducer(&self.current.get());
        if self.history_index + 1 < self.history.len() {
            self.history.truncate(self.history_index + 1);
        }
        self.history.push(next_state.clone());
        self.history_index += 1;
        self.current.set(next_state);
    }

    /// Undo to previous state snapshot.
    pub fn undo(&mut self) -> bool {
        if self.history_index > 0 {
            self.history_index -= 1;
            self.current.set(self.history[self.history_index].clone());
            true
        } else {
            false
        }
    }

    /// Redo to next state snapshot.
    pub fn redo(&mut self) -> bool {
        if self.history_index + 1 < self.history.len() {
            self.history_index += 1;
            self.current.set(self.history[self.history_index].clone());
            true
        } else {
            false
        }
    }

    pub fn history_depth(&self) -> usize {
        self.history.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct CounterState {
        count: i32,
    }

    #[test]
    fn test_state_store_dispatch_and_time_travel() {
        let mut store = StateStore::new(CounterState { count: 0 });
        assert_eq!(store.get().count, 0);

        store.dispatch(|s| CounterState {
            count: s.count + 10,
        });
        assert_eq!(store.get().count, 10);

        store.dispatch(|s| CounterState { count: s.count + 5 });
        assert_eq!(store.get().count, 15);

        // Undo 1 step
        assert!(store.undo());
        assert_eq!(store.get().count, 10);

        // Undo to initial
        assert!(store.undo());
        assert_eq!(store.get().count, 0);
        assert!(!store.undo()); // cannot undo further

        // Redo 1 step
        assert!(store.redo());
        assert_eq!(store.get().count, 10);
    }
}
