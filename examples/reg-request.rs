use ethercat::{AlState, Master, MasterAccess, SlaveAddr, SlaveId, SlavePos};
use ethercat_sys as ec;
use std::{io, thread, time::Duration};

pub fn main() -> Result<(), io::Error> {
    // This example demonstrates exchanging raw EtherCAT register contents
    // during cyclic operation.
    //
    // It is intended for debugging/monitoring. Register access is device- and
    // topology-specific; the address below is only an example.

    let mut master = Master::open(0, MasterAccess::ReadWrite)?;
    master.reserve()?;

    // `activate()` requires at least one domain.
    let domain_idx = master.create_domain()?;

    let slave_pos = SlavePos::from(0u16);
    master.request_state(slave_pos, AlState::PreOp)?;

    // Configure the first slave (position 0). Replace IDs with your device.
    let mut reg = {
        let mut cfg = master.configure_slave(
            SlaveAddr::ByPos(0),
            SlaveId {
                vendor_id: 0,
                product_code: 0,
            },
        )?;

        // Create a register request with a 16-byte buffer.
        cfg.create_reg_request(16)?
    };

    master.activate()?;

    // Example register address (device-dependent).
    const ADDRESS: u16 = 0x0110;

    // Trigger the first read.
    reg.read(ADDRESS, 2)?;

    let cycle_time = Duration::from_millis(10);
    loop {
        master.receive()?;
        master.domain(domain_idx).process()?;
        master.domain(domain_idx).queue()?;

        match reg.state()? {
            ec::EC_REQUEST_SUCCESS => {
                println!("Register data: {:02X?}", &reg.data()[..2]);
                reg.read(ADDRESS, 2)?;
            }
            ec::EC_REQUEST_ERROR => {
                eprintln!("Register request failed; retrying");
                reg.read(ADDRESS, 2)?;
            }
            ec::EC_REQUEST_UNUSED | ec::EC_REQUEST_BUSY => {
                // No completed result yet.
            }
            _ => {}
        }

        master.send()?;
        thread::sleep(cycle_time);
    }
}
