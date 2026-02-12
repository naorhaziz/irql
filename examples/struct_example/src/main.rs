//! Struct methods with IRQL constraints via impl blocks.

use irql::*;

struct Device {
    _name: &'static str,
}

#[irql(max = Dispatch)]
impl Device {
    fn new(name: &'static str) -> Self {
        Device { _name: name }
    }

    fn process_interrupt(&self) {
        // Runs at Dispatch or below.
    }
}

struct Driver {
    device: Device,
}

#[irql(max = Passive)]
impl Driver {
    fn new(device_name: &'static str) -> Self {
        let device = call_irql!(Device::new(device_name));
        Driver { device }
    }

    fn start(&self) {
        call_irql!(self.device.process_interrupt());
    }
}

#[irql(at = Passive)]
fn main() {
    let driver = call_irql!(Driver::new("example_device"));
    call_irql!(driver.start());
}
