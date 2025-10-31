use crate::rendering_backends::video::utils::FrameArray;
use anyhow::anyhow;
use ffmpeg_next::ffi::{AVPixelFormat, av_image_copy_to_buffer};
use ffmpeg_next::frame::video::Video as VideoFrame;
use std::sync::{Arc, Mutex};

pub struct FramePoolHandle {
    buffer: Option<Box<FrameArray>>,
    pool: Arc<Mutex<Vec<Box<FrameArray>>>>,
}

impl FramePoolHandle {
    fn new(buffer: Box<FrameArray>, pool: Arc<Mutex<Vec<Box<FrameArray>>>>) -> Self {
        Self {
            buffer: Some(buffer),
            pool,
        }
    }

    pub fn buffer(&self) -> &FrameArray {
        self.buffer
            .as_ref()
            .expect("FramePoolHandle buffer accessed after drop - this is a bug")
    }

    pub fn buffer_mut(&mut self) -> &mut FrameArray {
        self.buffer
            .as_mut()
            .expect("FramePoolHandle buffer accessed after drop - this is a bug")
    }

    pub fn fill_with(&mut self, frame: &mut VideoFrame) -> anyhow::Result<()> {
        let (width, height) = (frame.width() as usize, frame.height() as usize);

        let buffer = self.buffer_mut();

        // Safety check: ensure buffer dimensions match frame dimensions
        if buffer.shape() != [height, width, 3] {
            return Err(anyhow!(
                "Buffer shape {:?} does not match frame dimensions [height: {}, width: {}]",
                buffer.shape(),
                height,
                width
            ));
        }

        unsafe {
            let frame_ptr = frame.as_mut_ptr();
            let frame_format =
                std::mem::transmute::<std::ffi::c_int, AVPixelFormat>((*frame_ptr).format);

            let bytes_copied = av_image_copy_to_buffer(
                buffer.as_mut_ptr(),
                buffer.len() as i32,
                (*frame_ptr).data.as_ptr() as *const *const u8,
                (*frame_ptr).linesize.as_ptr(),
                frame_format,
                width as i32,
                height as i32,
                1,
            );

            if bytes_copied == buffer.len() as i32 {
                Ok(())
            } else {
                Err(anyhow!("Error copying frame data to buffer"))
            }
        }
    }
}

impl Drop for FramePoolHandle {
    fn drop(&mut self) {
        if let Some(buffer) = self.buffer.take() {
            if let Ok(mut pool) = self.pool.lock() {
                pool.push(buffer);
            }
        }
    }
}

pub struct FramePool {
    buffers: Arc<Mutex<Vec<Box<FrameArray>>>>,
    width: usize,
    height: usize,
}

impl FramePool {
    pub fn new(width: usize, height: usize, capacity: usize) -> Self {
        let mut buffers = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            buffers.push(Box::new(FrameArray::default((height, width, 3))));
        }
        Self {
            buffers: Arc::new(Mutex::new(buffers)),
            width,
            height,
        }
    }

    pub fn get_buffer(&mut self) -> FramePoolHandle {
        let mut buffers = self.buffers.lock().unwrap();

        if let Some(buffer) = buffers.pop() {
            FramePoolHandle::new(buffer, Arc::clone(&self.buffers))
        } else {
            tracing::warn!("Frame buffer pool is empty, creating a new buffer");
            let new_buffer = Box::new(FrameArray::default((self.height, self.width, 3)));
            FramePoolHandle::new(new_buffer, Arc::clone(&self.buffers))
        }
    }
}
