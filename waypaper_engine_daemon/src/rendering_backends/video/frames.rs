use std::cmp::Ordering;
use std::collections::BinaryHeap;
use ffmpeg_next::frame::Video;


struct TimedVideoFrame(Video);

impl Ord for TimedVideoFrame {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.timestamp().cmp(&self.0.timestamp())
    }
}

struct OrderedFramesContainer {
    frames: BinaryHeap<Video>
}

impl OrderedFramesContainer {
    fn push(&mut self, frame: Video) {
        self.frames.push(frame);
    }
}