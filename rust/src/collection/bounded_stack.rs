use std::collections::VecDeque;

#[derive(Clone)]
pub struct BoundedStack<T> {
    deque: VecDeque<T>,
    capacity: usize,
}

#[allow(dead_code)]
impl<T> BoundedStack<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            deque: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, item: T) -> Option<T> {
        let removed_item = if self.deque.len() == self.capacity {
            // Remove the oldest element (from the front)
            self.deque.pop_front()
        } else {
            None
        };

        // Add the new element to the top of the stack (the back)
        self.deque.push_back(item);
        removed_item
    }

    pub fn pop(&mut self) -> Option<T> {
        let result = self.deque.pop_back();
        self.deque.shrink_to_fit();
        result
    }

    pub fn peek(&self) -> Option<&T> {
        self.deque.back()
    }

    pub fn is_empty(&self) -> bool {
        self.deque.is_empty()
    }

    pub fn into_vec(self) -> Vec<T> {
        self.deque.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_within_capacity() {
        let mut stack = BoundedStack::new(3);
        assert_eq!(stack.push(1), None);
        assert_eq!(stack.push(2), None);
        assert_eq!(stack.push(3), None);

        assert_eq!(stack.pop(), Some(3));
        assert_eq!(stack.pop(), Some(2));
        assert_eq!(stack.pop(), Some(1));
        assert_eq!(stack.pop(), None);
    }

    #[test]
    fn test_push_beyond_capacity() {
        let mut stack = BoundedStack::new(3);
        assert_eq!(stack.push(1), None);
        assert_eq!(stack.push(2), None);
        assert_eq!(stack.push(3), None);
        assert_eq!(stack.push(4), Some(1)); // This should return Some(1) as it is pushed out

        assert_eq!(stack.pop(), Some(4));
        assert_eq!(stack.pop(), Some(3));
        assert_eq!(stack.pop(), Some(2));
        assert_eq!(stack.pop(), None);
    }

    #[test]
    fn test_peek() {
        let mut stack = BoundedStack::new(3);
        stack.push(1);
        stack.push(2);
        stack.push(3);

        assert_eq!(stack.peek(), Some(&3));
        stack.push(4); // Pushes out 1
        assert_eq!(stack.peek(), Some(&4));
    }

    #[test]
    fn test_is_empty() {
        let mut stack = BoundedStack::new(3);
        assert!(stack.is_empty());
        stack.push(1);
        assert!(!stack.is_empty());
        stack.pop();
        assert!(stack.is_empty());
    }

    #[test]
    fn test_push_and_pop_mixed() {
        let mut stack = BoundedStack::new(2);
        assert_eq!(stack.push(1), None);
        assert_eq!(stack.push(2), None);
        assert_eq!(stack.pop(), Some(2));

        assert_eq!(stack.push(3), None);
        assert_eq!(stack.push(4), Some(1)); // 1 should be pushed out
        assert_eq!(stack.pop(), Some(4));
        assert_eq!(stack.pop(), Some(3));
        assert_eq!(stack.pop(), None);
    }
}
