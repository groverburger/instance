//! Bounded undo/redo stack of whole states.

/// Undo history: `push` the state before every change.
#[derive(Debug, Clone)]
pub struct History<T> {
    undo: Vec<T>,
    redo: Vec<T>,
    limit: usize,
}

impl<T: Clone> History<T> {
    pub fn new(limit: usize) -> Self {
        History {
            undo: Vec::new(),
            redo: Vec::new(),
            limit: limit.max(1),
        }
    }

    /// Record `before` (the state prior to a change); clears the redo list.
    pub fn push(&mut self, before: T) {
        self.undo.push(before);
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// Return the previous state; `current` goes onto the redo list.
    pub fn undo(&mut self, current: T) -> Option<T> {
        let prev = self.undo.pop()?;
        self.redo.push(current);
        Some(prev)
    }

    /// Return the next state; `current` goes onto the undo list.
    pub fn redo(&mut self, current: T) -> Option<T> {
        let next = self.redo.pop()?;
        self.undo.push(current);
        Some(next)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}
