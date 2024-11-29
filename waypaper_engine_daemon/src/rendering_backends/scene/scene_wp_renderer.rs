use std::rc::Rc;
use smithay_client_toolkit::reexports::client::Connection;
use waypaper_engine_shared::project::WallpaperType;
use crate::egl::EGLState;
use crate::rendering_backends::scene::scene_structs::Scene;
use crate::wallpaper::Wallpaper;
use crate::wallpaper_renderer::WPRendererImpl;

pub(crate) struct SceneWPRenderer {
    _connection: Rc<Connection>,
    _egl_state: Rc<EGLState>,
    render_context: Option<RenderContext>,
}


impl SceneWPRenderer {
    pub(crate) fn new(connection: Rc<Connection>, egl_state: Rc<EGLState>) -> Self {
        Self {
            _connection: connection,
            _egl_state: egl_state,
            render_context: None,
        }
    }
}

struct RenderContext {
    scene: Scene,
}

impl WPRendererImpl for SceneWPRenderer {

    fn init_render(&mut self) {
        
    }

    fn setup_wallpaper(&mut self, wp: &Wallpaper) {
        match wp {
            Wallpaper::Scene { scene_package, .. } => {
                let scene_json = scene_package.get_file("scene.json").expect("Couldn't find scene.json file");
                let scene: Scene = serde_json::from_slice(scene_json.bytes()).expect("Couldn't parse scene.json");
                
                tracing::debug!("{:?}", scene);
                self.render_context = Some(RenderContext { scene });
            },
            _ => unreachable!(),
        }
    }

    fn render(&mut self, width: u32, height: u32) {
        
    }

    fn get_wp_type(&self) -> WallpaperType {
        WallpaperType::Scene
    }

    fn clear_color(&self) -> (f32, f32, f32) {
        if let Some(render_context) = self.render_context.as_ref() {
            let clear_color = render_context.scene.general.ambientcolor;
            (clear_color.0 as f32, clear_color.1 as f32, clear_color.2 as f32)
        } else {
            (0.0, 0.0, 0.0)
        }
    }
}