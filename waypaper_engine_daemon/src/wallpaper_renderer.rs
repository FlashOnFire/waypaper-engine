use std::rc::Rc;

use smithay_client_toolkit::reexports::client::Connection;

use waypaper_engine_shared::project::WallpaperType;

use crate::egl::EGLState;
use crate::rendering_backends::scene::scene_wp_renderer::SceneWPRenderer;
use crate::rendering_backends::video::video_wp_renderer::VideoWPRenderer;
use crate::wallpaper::Wallpaper;

pub struct WPRenderer {
    _connection: Rc<Connection>,
    _egl_state: Rc<EGLState>,
    renderer: Option<Box<dyn WPRendererImpl>>,
    renderer_initialized: bool,
}

impl WPRenderer {
    pub fn new(connection: Rc<Connection>, egl_state: Rc<EGLState>) -> Self {
        Self {
            _connection: connection,
            _egl_state: egl_state,
            renderer: None,
            renderer_initialized: false,
        }
    }

    pub fn setup_for(&mut self, wallpaper: &Wallpaper) {
        if self.renderer.is_none()
            || self.renderer.as_ref().unwrap().get_wp_type() != wallpaper.get_wp_type()
        {
            match wallpaper {
                Wallpaper::Video { .. } => {
                    self.renderer = Some(Box::new(VideoWPRenderer::new()));
                }
                Wallpaper::Scene { .. } => {
                    self.renderer = Some(Box::new(SceneWPRenderer::new()));
                }
                Wallpaper::Web { .. } => {}
                Wallpaper::Preset { .. } => {}
            }

            self.renderer_initialized = false;
        }

        self.renderer.as_mut().unwrap().setup_wallpaper(wallpaper);
    }

    pub(crate) fn clear_color(&self) -> (f32, f32, f32) {
        if let Some(renderer) = &self.renderer {
            renderer.clear_color()
        } else {
            (0.0, 0.0, 0.0)
        }
    }

    pub(crate) fn render(&mut self, width: u32, height: u32) {
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.render(width, height);
        } else {
            unreachable!();
        }
    }

    pub(crate) fn init_render(&mut self) {
        if !self.renderer_initialized {
            if let Some(renderer) = self.renderer.as_mut() {
                renderer.init_render();
                self.renderer_initialized = true;
            } else {
                unreachable!();
            }
        }
    }
}

pub(crate) trait WPRendererImpl {
    fn init_render(&mut self);

    fn setup_wallpaper(&mut self, wp: &Wallpaper);

    fn render(&mut self, width: u32, height: u32);

    fn get_wp_type(&self) -> WallpaperType;

    fn clear_color(&self) -> (f32, f32, f32) {
        (0.0, 0.0, 0.0)
    }
}
