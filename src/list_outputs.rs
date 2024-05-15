use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use smithay_client_toolkit::{delegate_output, delegate_registry, registry_handlers};
use smithay_client_toolkit::output::{OutputHandler, OutputInfo, OutputState};
use smithay_client_toolkit::reexports::client::{Connection, EventQueue, QueueHandle};
use smithay_client_toolkit::reexports::client::globals::registry_queue_init;
use smithay_client_toolkit::reexports::client::protocol::wl_output;
use smithay_client_toolkit::reexports::client::protocol::wl_output::WlOutput;
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};

pub struct ListOutputs {
    registry_state: RegistryState,
    event_queue: Rc<RefCell<EventQueue<ListOutputs>>>,
    output_state: OutputState,
}

impl ListOutputs {
    pub fn new(conn: &Connection) -> Self {
        let (globals, event_queue) = registry_queue_init(conn).unwrap();
        let qh = event_queue.handle();

        let output = OutputState::new(&globals, &qh);
        
        ListOutputs {
            registry_state: RegistryState::new(&globals),
            event_queue: Rc::new(RefCell::new(event_queue)),
            output_state: output,
        }
    }

    pub fn get_outputs(&mut self) -> OutputsList {
        self.event_queue.clone().borrow_mut().roundtrip(self).unwrap();

        OutputsList(self.output_state.outputs().map(|output| (output.clone(), self.output_state.info(&output).unwrap())).collect())
    }
}

pub struct OutputsList(HashMap<WlOutput, OutputInfo>);

impl OutputsList {
    pub fn print_outputs(&self) {
        for info in self.0.values() {
            let name = info.name.clone().expect("Output doesn't have a name");

            let current_mode = info.modes.iter().find(|mode| mode.current).expect("Couldn't find output current mode");
            let (width, height) = current_mode.dimensions;
            let refresh_rate = (current_mode.refresh_rate as f32 / 1000.0).ceil() as i32;

            println!("Outputs :");
            println!("\t- {:} : {}x{} - {}hz", name, width, height, refresh_rate);
        }
    }
}

impl Deref for OutputsList {
    type Target = HashMap<WlOutput, OutputInfo>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for OutputsList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl OutputHandler for ListOutputs {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
        println!("{:?}", _output);
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {}

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {}
}

delegate_output!(ListOutputs);
delegate_registry!(ListOutputs);

impl ProvidesRegistryState for ListOutputs {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}