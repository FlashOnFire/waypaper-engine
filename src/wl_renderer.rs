use std::path::PathBuf;
use std::rc::Rc;

use gl::COLOR_BUFFER_BIT;
use khronos_egl::{ATTRIB_NONE, Context, Surface};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_layer, delegate_output,
    delegate_registry, delegate_seat, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
    },
    shell::{
        WaylandSurface,
        wlr_layer::{
            KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
    },
    shm::{Shm, ShmHandler},
};
use smithay_client_toolkit::output::OutputInfo;
use smithay_client_toolkit::reexports::client::{Connection, EventQueue, Proxy, QueueHandle};
use smithay_client_toolkit::reexports::client::globals::{GlobalList, registry_queue_init};
use smithay_client_toolkit::reexports::client::protocol::{wl_output, wl_seat, wl_surface};
use smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput;
use smithay_client_toolkit::shell::wlr_layer::Anchor;
use wayland_egl::WlEglSurface;

use crate::egl::EGLState;
use crate::mpv::MpvRenderer;

pub struct WLState {
    pub connection: Rc<Connection>,
    _globals: GlobalList,
    event_queue: EventQueue<SimpleLayer>,
    _queue_handle: QueueHandle<SimpleLayer>,
    _compositor: CompositorState,
    _layer_shell: LayerShell,

    pub simple_layer: SimpleLayer,
}

impl WLState {
    pub fn new(connection: Rc<Connection>, output: (&WlOutput, &OutputInfo), file: PathBuf, egl_state: Rc<EGLState>) -> Self {
        let (globals, event_queue) = registry_queue_init(&connection).unwrap();
        let qh = event_queue.handle();

        let output_state = OutputState::new(&globals, &qh);

        let compositor = CompositorState::bind(&globals, &qh).expect("wl_compositor is not available");
        let layer_shell = LayerShell::bind(&globals, &qh).expect("layer shell is not available");
        let shm = Shm::bind(&globals, &qh).expect("wl_shm is not available");

        let surface = compositor.create_surface(&qh);
        
        let output_size = output.1.logical_size.unwrap();

        let layer =
            layer_shell.create_layer_surface(&qh, surface, Layer::Background, Some("waypaper_engine"), Some(output.0));
        layer.set_size(output_size.0 as u32, output_size.1 as u32);
        layer.set_anchor(Anchor::BOTTOM | Anchor::TOP | Anchor::LEFT | Anchor::RIGHT);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);

        layer.commit();

        connection.roundtrip().unwrap();

        let wl_egl_surface = WlEglSurface::new(
            layer.wl_surface().id(),
            output_size.0,
            output_size.1,
        ).unwrap();

        let egl_window_surface =
            unsafe {
                egl_state.egl.create_platform_window_surface(
                    egl_state.egl_display,
                    egl_state.config,
                    wl_egl_surface.ptr() as khronos_egl::NativeWindowType,
                    &[ATTRIB_NONE],
                )
            }.expect("Unable to create an EGL surface");

        layer.commit();
        connection.roundtrip().unwrap();
        connection.flush().unwrap();

        egl_state.egl.make_current(
            egl_state.egl_display,
            Some(egl_window_surface),
            Some(egl_window_surface),
            Some(egl_state.egl_context),
        ).unwrap();

        let simple_layer = SimpleLayer {
            registry_state: RegistryState::new(&globals),
            seat_state: SeatState::new(&globals, &qh),
            output_state,
            shm,

            exit: false,
            first_configure: true,
            width: 256,
            height: 256,
            layer,
            //keyboard: None,
            //keyboard_focus: false,
            //pointer: None,
            mpv_renderer: None,
            egl_state,
            _wl_egl_surface: wl_egl_surface,
            egl_window_surface,
        };

        let mut wl_state = WLState {
            connection: connection.clone(),
            _globals: globals,
            event_queue,
            _queue_handle: qh,
            _compositor: compositor,
            _layer_shell: layer_shell,

            simple_layer,
        };

        let mpv_renderer = MpvRenderer::new(connection, file);

        wl_state.simple_layer.mpv_renderer = Some(mpv_renderer);

        wl_state
    }

    pub(crate) fn loop_fn(&mut self) {
        loop {
            self.event_queue.blocking_dispatch(&mut self.simple_layer).unwrap();

            if self.simple_layer.exit {
                println!("exiting layer");
                break;
            }
        }
    }
}

pub struct SimpleLayer {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    shm: Shm,

    exit: bool,
    first_configure: bool,
    width: u32,
    height: u32,
    pub(crate) layer: LayerSurface,
    //keyboard: Option<wl_keyboard::WlKeyboard>,
    //keyboard_focus: bool,
    //pointer: Option<wl_pointer::WlPointer>,
    mpv_renderer: Option<MpvRenderer>,
    egl_state: Rc<EGLState>,
    _wl_egl_surface: WlEglSurface,
    egl_window_surface: Surface,

}

impl CompositorHandler for SimpleLayer {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {        
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {}

    fn frame(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.draw(qh);
    }
}

impl OutputHandler for SimpleLayer {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: WlOutput,
    ) {}

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: WlOutput,
    ) {}

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: WlOutput,
    ) {}
}

impl LayerShellHandler for SimpleLayer {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        println!("New Size : {:?}", configure.new_size);

        if configure.new_size.0 == 0 || configure.new_size.1 == 0 {
            self.width = 1280;
            self.height = 720;
        } else {
            self.width = configure.new_size.0;
            self.height = configure.new_size.1;
        }

        // Initiate the first draw.
        if self.first_configure {
            self.first_configure = false;
            self.draw(qh);
        }
    }
}

impl SeatHandler for SimpleLayer {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        _capability: Capability,
    ) {
        /*if capability == Capability::Keyboard && self.keyboard.is_none() {
            println!("Set keyboard capability");
            let keyboard =
                self.seat_state.get_keyboard(qh, &seat, None).expect("Failed to create keyboard");
            self.keyboard = Some(keyboard);
        }

        if capability == Capability::Pointer && self.pointer.is_none() {
            println!("Set pointer capability");
            let pointer = self.seat_state.get_pointer(qh, &seat).expect("Failed to create pointer");
            self.pointer = Some(pointer);
        }*/
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        _capability: Capability,
    ) {
        /*if capability == Capability::Keyboard && self.keyboard.is_some() {
            println!("Unset keyboard capability");
            self.keyboard.take().unwrap().release();
        }

        if capability == Capability::Pointer && self.pointer.is_some() {
            println!("Unset pointer capability");
            self.pointer.take().unwrap().release();
        }*/
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl ShmHandler for SimpleLayer {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl SimpleLayer {
    pub fn draw(&mut self, qh: &QueueHandle<Self>) {
        let width = self.width;
        let height = self.height;

        // Draw to the window:
        unsafe {
            gl::ClearColor(1.0,1.0,1.0,1.0);
            gl::Clear(COLOR_BUFFER_BIT);
            gl::Flush();

            self.mpv_renderer.as_mut().unwrap().render_context.render::<Context>(0, self.width as i32, self.height as i32, true).unwrap();
        }

        self.egl_state.egl.make_current(
            self.egl_state.egl_display,
            Some(self.egl_window_surface),
            Some(self.egl_window_surface),
            Some(self.egl_state.egl_context),
        ).unwrap();

        // Damage the entire window
        self.layer.wl_surface().damage_buffer(0, 0, width as i32, height as i32);

        // Request our next frame
        self.layer.wl_surface().frame(qh, self.layer.wl_surface().clone());

        self.egl_state.egl.swap_buffers(self.egl_state.egl_display, self.egl_window_surface).unwrap();

        // Attach and commit to present.
        self.layer.commit();
    }
}

delegate_compositor!(SimpleLayer);
delegate_output!(SimpleLayer);
delegate_shm!(SimpleLayer);
delegate_seat!(SimpleLayer);
delegate_layer!(SimpleLayer);
delegate_registry!(SimpleLayer);

impl ProvidesRegistryState for SimpleLayer {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState];
}

