use crate::rendering_backends::video::frame_pool::FramePoolHandle;
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

pub struct TimedVideoFrame {
    pub(crate) frame: FramePoolHandle,
    pub(crate) rewind_count: u32,
    pub(crate) timestamp: f32,
}

impl TimedVideoFrame {
    pub fn new(frame: FramePoolHandle, timestamp: f32, rewind_count: u32) -> Self {
        Self {
            frame,
            rewind_count,
            timestamp,
        }
    }
}

impl PartialEq for TimedVideoFrame {
    fn eq(&self, other: &Self) -> bool {
        self.rewind_count == other.rewind_count && self.timestamp == other.timestamp
    }
}

impl Eq for TimedVideoFrame {}

impl Ord for TimedVideoFrame {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.rewind_count.cmp(&other.rewind_count) {
            Ordering::Equal => self.timestamp.total_cmp(&other.timestamp),
            other => other,
        }
    }
}

impl PartialOrd for TimedVideoFrame {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct OrderedFramesContainer<T: Ord> {
    frames: BinaryHeap<Reverse<T>>,
}

impl<T> OrderedFramesContainer<T>
where
    T: Ord,
{
    pub fn new() -> Self {
        Self {
            frames: BinaryHeap::new(),
        }
    }
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            frames: BinaryHeap::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, frame: T) {
        if self.frames.iter().any(|Reverse(f)| f.eq(&frame)) {
            tracing::warn!("Frame already exists in the queue, skipping push");
            return;
        }

        tracing::info!(
            "capacity: {}, len: {}",
            self.frames.capacity(),
            self.frames.len()
        );

        self.frames.push(Reverse(frame));
    }

    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        for frame in iter {
            self.push(frame);
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        self.frames.pop().map(|Reverse(frame)| frame)
    }

    pub fn peek(&self) -> Option<&T> {
        self.frames.peek().map(|Reverse(frame)| frame)
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn clear(&mut self) {
        self.frames.clear();
    }
}
