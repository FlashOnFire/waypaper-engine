use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use std::rc::Rc;

use smithay_client_toolkit::reexports::client::Connection;

use crate::egl::EGLState;
use crate::rendering_backends::scene::scene_wp_renderer::SceneWPRenderer;
use crate::rendering_backends::video::video_wp_renderer::VideoWPRenderer;
use crate::scene_package::ScenePackage;
use crate::wallpaper::Wallpaper;

pub struct WPRenderer {
    _connection: Rc<Connection>,
    _egl_state: Rc<EGLState>,
    renderer: Option<RenderingBackend>,
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

    pub fn setup_wallpaper(&mut self, wallpaper: &Wallpaper) {
        match wallpaper {
            Wallpaper::Video {
                project,
                base_dir_path,
            } => {
                if let Some(RenderingBackend::Video(video_renderer)) = &mut self.renderer {
                    video_renderer
                        .setup_video_wallpaper(base_dir_path.join(project.file.as_ref().unwrap()));
                } else {
                    let mut renderer = Box::new(VideoWPRenderer::new());
                    renderer
                        .setup_video_wallpaper(base_dir_path.join(project.file.as_ref().unwrap()));
                    self.renderer = Some(RenderingBackend::Video(renderer));
                }
            }
            Wallpaper::Scene { scene_package, .. } => {
                if let Some(RenderingBackend::Scene(scene_renderer)) = &mut self.renderer {
                    scene_renderer.setup_scene_wallpaper(scene_package);
                } else {
                    let mut renderer = Box::new(SceneWPRenderer::new());
                    renderer.setup_scene_wallpaper(scene_package);
                    self.renderer = Some(RenderingBackend::Scene(renderer));
                }
            }
            Wallpaper::Web { .. } => todo!(),
            Wallpaper::Preset { .. } => todo!(),
        }
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

    fn render(&mut self, width: u32, height: u32);

    fn clear_color(&self) -> (f32, f32, f32) {
        (0.0, 0.0, 0.0)
    }
}

pub(crate) trait VideoRenderingBackend: WPRendererImpl {
    fn setup_video_wallpaper(&mut self, video_path: PathBuf);
}

pub(crate) trait SceneRenderingBackend: WPRendererImpl {
    fn setup_scene_wallpaper(&mut self, scene_package: &ScenePackage);
}

enum RenderingBackend {
    Video(Box<dyn VideoRenderingBackend>),
    Scene(Box<dyn SceneRenderingBackend>),
}

impl Deref for RenderingBackend {
    type Target = dyn WPRendererImpl;

    fn deref(&self) -> &Self::Target {
        match self {
            RenderingBackend::Video(renderer) => renderer.as_ref(),
            RenderingBackend::Scene(renderer) => renderer.as_ref(),
        }
    }
}

impl DerefMut for RenderingBackend {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            RenderingBackend::Video(renderer) => renderer.as_mut(),
            RenderingBackend::Scene(renderer) => renderer.as_mut(),
        }
    }
}
