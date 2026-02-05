use irql::{Dispatch, Passive, requires_irql, root_irql};

struct Device {
    _name: &'static str,
}

#[requires_irql(Dispatch)]
impl Device {
    fn new(name: &'static str) -> Self {
        Device { _name: name }
    }

    fn process_interrupt(&self) {
        // Processing interrupt at Dispatch level
    }
}

struct Driver {
    device: Device,
}

#[requires_irql(Passive)]
impl Driver {
    fn new(device_name: &'static str) -> Self {
        // Create device at Dispatch level from Passive context
        let device = call_irql!(Device::new(device_name));
        Driver { device }
    }

    fn start(&self) {
        // At Passive level, can raise to Dispatch
        call_irql!(self.device.process_interrupt());
    }
}

#[root_irql(Passive)]
fn main() {
    let driver = call_irql!(Driver::new("example_device"));
    call_irql!(driver.start());
}
