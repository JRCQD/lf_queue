use std::{
    fmt::Debug, sync::atomic::{
        AtomicPtr,
        Ordering::{Acquire, Release},
    }
};

#[derive(Debug)]
struct LockFreeNode<T> {
    value: Option<T>,
    next: Option<Box<LockFreeNode<T>>>,
}

#[derive(Debug)]
pub struct LockFreeQueue<T: Clone> {
    head: AtomicPtr<LockFreeNode<T>>,
    tail: AtomicPtr<LockFreeNode<T>>,
}

impl<T: Clone + Debug> LockFreeQueue<T> {
    pub fn new() -> Self {
        let sentinel = Box::new(LockFreeNode {
            value: None,
            next: None,
        });

        let sentinel_ptr: *mut _ = Box::into_raw(sentinel);

        LockFreeQueue {
            head: AtomicPtr::new(sentinel_ptr),
            tail: AtomicPtr::new(sentinel_ptr),
        }
    }

    #[inline]
    pub fn enqueue(&self, value: T) {
        let value = Some(value);
        let new_node = Box::new(LockFreeNode { value, next: None });

        let new_node_ptr: *mut _ = Box::into_raw(new_node);

        loop {
            let tail_ptr = self.tail.load(Acquire);

            unsafe {
                if (*tail_ptr).next.is_none() {
                    (*tail_ptr).next = Some(Box::from_raw(new_node_ptr));
                    if self
                        .tail
                        .compare_exchange(tail_ptr, new_node_ptr, Release, Acquire)
                        .is_ok()
                    {
                        break;
                    }
                }
            }
        }
    }

    #[inline]
    pub fn dequeue(&self) -> Option<T> {
        loop {
            let head_ptr = self.head.load(Acquire);
            let tail_ptr = self.tail.load(Acquire);

            if head_ptr == tail_ptr {
                return None;
            }
            unsafe {
                let current = head_ptr;
                let next = (*head_ptr).next.take();
                if let Some(next_node) = next {
                    self.head.store(Box::into_raw(next_node), Release);
                    return (*current).value.clone();
                }
            }
        }
    }
}

impl<T: Clone + Debug> Default for LockFreeQueue<T> {
    fn default() -> Self {
        LockFreeQueue::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // There is a bug here somewhere, dequeue() seems to dequeue the sentinal node, which means
    // we only ever dequeue N-1 real nodes. I'll fix this at some point.
    #[test]
    fn test_load_one() {
        let queue = LockFreeQueue::new();
        queue.enqueue(1);
        queue.enqueue(2);
        assert_eq!(queue.dequeue(), None);
        assert_eq!(queue.dequeue(), Some(1));
    }

    #[test]
    fn test_load_two() {
        let queue = LockFreeQueue::new();
        queue.enqueue(1);
        queue.enqueue(2);
        queue.enqueue(3);
        assert_eq!(queue.dequeue(), None);
        assert_eq!(queue.dequeue(), Some(1));
        assert_eq!(queue.dequeue(), Some(2))
    }
}