use ffmpeg_next::frame::Video;
use std::cmp::{Ordering, Reverse};
use std::collections::BinaryHeap;

struct TimedVideoFrame {
    rewind_count: u32,
    frame: Video,
}

impl TimedVideoFrame {
    pub fn from(frame: Video, rewind_count: u32) -> Self {
        Self {
            rewind_count,
            frame,
        }
    }
}

impl PartialEq for TimedVideoFrame {
    fn eq(&self, other: &Self) -> bool {
        self.rewind_count == other.rewind_count && self.frame.timestamp() == other.frame.timestamp()
    }
}

impl Eq for TimedVideoFrame {}

impl Ord for TimedVideoFrame {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.rewind_count.cmp(&other.rewind_count) {
            Ordering::Equal => self.frame.timestamp().cmp(&other.frame.timestamp()),
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
        self.frames.push(Reverse(frame));
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
