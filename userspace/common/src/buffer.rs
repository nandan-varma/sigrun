//! Buffer utilities for userspace services

use core::sync::atomic::{AtomicUsize, Ordering};

pub struct RingBuffer<T, const N: usize>
where
    T: Copy,
{
    buffer: [Option<T>; N],
    head: AtomicUsize,
    tail: AtomicUsize,
}

impl<T, const N: usize> Default for RingBuffer<T, N>
where
    T: Copy,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> RingBuffer<T, N>
where
    T: Copy,
{
    pub const fn new() -> Self {
        Self {
            buffer: [None; N],
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
        }
    }

    pub fn push(&self, item: T) -> bool {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);

        let next_head = (head + 1) % N;
        if next_head == tail {
            return false;
        }

        unsafe {
            let ptr = self.buffer.as_ptr() as *mut Option<T>;
            (*ptr.add(head)) = Some(item);
        }
        self.head.store(next_head, Ordering::Release);
        true
    }

    pub fn pop(&self) -> Option<T> {
        let tail = self.tail.load(Ordering::Acquire);
        let head = self.head.load(Ordering::Acquire);

        if tail == head {
            return None;
        }

        let item = unsafe {
            let ptr = self.buffer.as_ptr() as *mut Option<T>;
            (*ptr.add(tail)).take()
        };

        self.tail.store((tail + 1) % N, Ordering::Release);
        item
    }

    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        if head >= tail {
            head - tail
        } else {
            N - tail + head
        }
    }

    pub fn is_empty(&self) -> bool {
        self.head.load(Ordering::Acquire) == self.tail.load(Ordering::Acquire)
    }

    pub fn is_full(&self) -> bool {
        let next_head = (self.head.load(Ordering::Acquire) + 1) % N;
        next_head == self.tail.load(Ordering::Acquire)
    }
}

pub struct ByteArray<const N: usize> {
    data: [u8; N],
    len: usize,
}

impl<const N: usize> Default for ByteArray<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> ByteArray<N> {
    pub const fn new() -> Self {
        Self {
            data: [0; N],
            len: 0,
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.len]
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data[..self.len]
    }

    pub fn append(&mut self, data: &[u8]) -> usize {
        let available = N - self.len;
        let to_copy = data.len().min(available);
        self.data[self.len..self.len + to_copy].copy_from_slice(&data[..to_copy]);
        self.len += to_copy;
        to_copy
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        N
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct IoVec {
    pub base: *mut u8,
    pub len: usize,
}

impl IoVec {
    pub const fn new(base: *mut u8, len: usize) -> Self {
        Self { base, len }
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.base, self.len) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.base, self.len) }
    }
}
