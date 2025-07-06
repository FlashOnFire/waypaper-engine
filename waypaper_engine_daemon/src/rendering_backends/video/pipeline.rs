use crate::rendering_backends::video::decoder::VideoDecoder;
use crate::rendering_backends::video::demuxer::{Demuxer, Packet};
use crate::rendering_backends::video::frames::{OrderedFramesContainer, TimedVideoFrame};
use crate::rendering_backends::video::utils;
use crate::rendering_backends::video::video_backend_consts::THREAD_FRAME_BUFFER_SIZE;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

// TODO: remove these arc mutexes and use channels instead
pub struct DecodingPipeline {
    demuxer: Arc<Mutex<Demuxer>>,
    video_decoder: Arc<Mutex<VideoDecoder>>,
    pub(crate) decoding_thread: Option<thread::JoinHandle<()>>,
    shutdown_flag: Arc<AtomicBool>,
    pub(crate) frames: Arc<Mutex<OrderedFramesContainer<TimedVideoFrame>>>,
}

impl DecodingPipeline {
    pub fn new(video_file: &Path) -> Self {
        let demuxer = Demuxer::new(video_file).expect("Failed to create demuxer");
        let video_stream = demuxer.video_stream().expect("Failed to find video stream");
        let video_decoder =
            VideoDecoder::new(&video_stream).expect("Failed to create video decoder");

        Self {
            demuxer: Arc::new(Mutex::new(demuxer)),
            video_decoder: Arc::new(Mutex::new(video_decoder)),
            decoding_thread: None,
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            frames: Arc::new(Mutex::new(OrderedFramesContainer::with_capacity(
                THREAD_FRAME_BUFFER_SIZE,
            ))),
        }
    }

    pub fn decoder_size(&self) -> (u32, u32) {
        self.video_decoder.lock().unwrap().size()
    }

    pub fn framerate(&self) -> f32 {
        let demuxer = self.demuxer.lock().unwrap();
        let video_stream = demuxer.video_stream().expect("No video stream found");

        (video_stream.avg_frame_rate().numerator() as f32)
            / (video_stream.avg_frame_rate().denominator() as f32)
    }

    pub fn start_decoding(&mut self) {
        let weak = Arc::downgrade(&self.frames);
        let shutdown_flag = Arc::clone(&self.shutdown_flag);
        let demuxer = Arc::clone(&self.demuxer);
        let video_decoder = Arc::clone(&self.video_decoder);

        self.decoding_thread = Some(thread::spawn(move || {
            let mut rewind_count: u32 = 0;
                'outer: while !shutdown_flag.load(Ordering::Relaxed) {
                    let mut demuxer = demuxer.lock().unwrap();
                    let packet = loop {
                        let packet_result: Option<Packet> = demuxer.read();

                        match packet_result {
                            Some(timed_packet) => match timed_packet {
                                Packet::Video(packet) => break packet,
                                _ => continue, // Skip non-video packets
                            },
                            None => {
                                tracing::debug!("Video ended, seeking to start");
                                demuxer.seek_to_start().unwrap();
                                rewind_count += 1;
                                continue;
                            }
                        };
                    };

                    tracing::debug!(
                        "Decoding packet with PTS: {}, pkt time base: {:?}",
                        packet.pts().unwrap(),
                        packet.time_base()
                    );

                    // Feed the packet to the video decoder
                    let mut video_decoder = video_decoder.lock().unwrap();
                    if let Err(e) = video_decoder.feed(packet) {
                        tracing::error!("Failed to feed packet to video decoder: {}", e);
                        continue;
                    }

                    let timed_frames = match video_decoder.receive_frames() {
                        Ok(Some(frames)) => {
                            tracing::debug!("Received {} frames from video decoder", frames.len());

                            frames.into_iter().map(|mut frame| {
                                let timestamp = frame.timestamp().or(frame.pts()).unwrap_or(0);

                                let tb = video_decoder.stream_time_base();
                                let tf = TimedVideoFrame::new(
                                    utils::convert_frame_to_ndarray_rgb24(&mut frame).unwrap(),
                                    timestamp as f32
                                        * (tb.numerator() as f32 / tb.denominator() as f32),
                                    rewind_count,
                                );

                                tracing::debug!(
                                    "Converted frame to ndarray, size: {}x{}, rewind count: {}, frame time: {:?}",
                                    frame.width(),
                                    frame.height(),
                                    tf.rewind_count,
                                    tf.timestamp
                                );
                                tf
                            }).collect::<Vec<_>>()
                        }
                        Ok(None) => {
                            tracing::debug!("No frame received, waiting for more data");
                            continue;
                        }
                        Err(e) => {
                            tracing::error!("Failed to decode video frame: {}", e);
                            continue;
                        }
                    };

                    while !shutdown_flag.load(Ordering::Relaxed) {
                        if let Some(strong) = weak.upgrade() {
                            if strong.lock().unwrap().len() >= THREAD_FRAME_BUFFER_SIZE {
                                tracing::debug!("Frames in queue >= 20, paused decoding");
                                thread::park();
                                tracing::debug!("Resumed decoding")
                            } else {
                                break;
                            };
                        } else {
                            break 'outer;
                        }
                    }

                    if let Some(strong) = weak.upgrade() {
                        let mut frames_vec = strong.lock().unwrap();
                        let nb_new_frames = timed_frames.len();
                        frames_vec.extend(timed_frames);
                        tracing::debug!(
                            "Added {} frames to the queue, total frames in queue: {}",
                            nb_new_frames,
                            frames_vec.len()
                        );
                    } else {
                        break 'outer;
                    }
                }
            tracing::info!("Decoding thread stopped");
            video_decoder.lock().unwrap().drain();
        }));
    }

    pub fn stop_decoding(&mut self) {
        self.shutdown_flag.store(true, Ordering::Relaxed);

        if let Some(thread) = self.decoding_thread.take() {
            thread.join().expect("Failed to join decoding thread");
        }
    }
}

impl Drop for DecodingPipeline {
    fn drop(&mut self) {
        tracing::info!("Dropping DecodingPipeline, stopping decoding thread");
        if self.decoding_thread.is_some() {
            self.stop_decoding();
        }
    }
}
