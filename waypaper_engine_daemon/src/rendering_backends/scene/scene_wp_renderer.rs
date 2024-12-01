use crate::egl::EGLState;
use crate::rendering_backends::scene::scene_structs::{Material, Model, ObjectValue, Scene};
use crate::tex_file::TexFile;
use crate::wallpaper::Wallpaper;
use crate::wallpaper_renderer::WPRendererImpl;
use smithay_client_toolkit::reexports::client::Connection;
use std::rc::Rc;
use waypaper_engine_shared::project::WallpaperType;

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
    texture: TexFile,
}

impl WPRendererImpl for SceneWPRenderer {
    fn init_render(&mut self) {}

    fn setup_wallpaper(&mut self, wp: &Wallpaper) {
        match wp {
            Wallpaper::Scene { scene_package, .. } => {
                let scene_json = scene_package
                    .get_file("scene.json")
                    .expect("Couldn't find scene.json file");
                let scene: Scene =
                    serde_json::from_slice(scene_json.bytes()).expect("Couldn't parse scene.json");

                let image = scene
                    .objects
                    .iter()
                    .find(|x| matches!(x.value, ObjectValue::Image { .. }))
                    .unwrap();

                tracing::info!("found image : {}", image.name);
                if let ObjectValue::Image { image, .. } = &image.value {
                    tracing::info!("Found model : {}", image);
                    let model: Model =
                        serde_json::from_slice(scene_package.contents.get(image).unwrap().bytes())
                            .unwrap();

                    tracing::info!("Found material : {}", model.material);
                    let material: Material = serde_json::from_slice(
                        scene_package.contents.get(&model.material).unwrap().bytes(),
                    )
                    .unwrap();

                    let first_pass = material.passes.first().unwrap();
                    let first_texture = first_pass.textures.first().unwrap();
                    let texture = TexFile::from_bytes(Vec::from(
                        scene_package
                            .contents
                            .get(&("materials/".to_owned() + first_texture + ".tex"))
                            .unwrap()
                            .bytes(),
                    ))
                    .unwrap();
                    tracing::debug!("{:?}", scene);
                    self.render_context = Some(RenderContext { scene, texture });
                }
            }
            _ => unreachable!(),
        }
    }

    fn render(&mut self, width: u32, height: u32) {}

    fn get_wp_type(&self) -> WallpaperType {
        WallpaperType::Scene
    }

    fn clear_color(&self) -> (f32, f32, f32) {
        if let Some(render_context) = self.render_context.as_ref() {
            let clear_color = render_context.scene.general.ambientcolor;
            (
                clear_color.0 as f32,
                clear_color.1 as f32,
                clear_color.2 as f32,
            )
        } else {
            (0.0, 0.0, 0.0)
        }
    }
}
