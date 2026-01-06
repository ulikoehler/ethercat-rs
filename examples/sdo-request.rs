use ethercat::{AlState, Idx, Master, MasterAccess, SdoIdx, SlaveAddr, SlaveId, SlavePos, SubIdx};
use ethercat_sys as ec;
use std::{io, thread, time::Duration};

pub fn main() -> Result<(), io::Error> {
    // This example demonstrates non-blocking SDO access suitable for
    // cyclic/realtime operation.
    //
    // It will only work on a system with an EtherCAT master kernel module and
    // at least one slave. The vendor/product IDs and SDO index below are
    // placeholders; adjust them for your device.

    let mut master = Master::open(0, MasterAccess::ReadWrite)?;
    master.reserve()?;

    // `activate()` requires at least one domain.
    let domain_idx = master.create_domain()?;

    let slave_pos = SlavePos::from(0u16);
    master.request_state(slave_pos, AlState::PreOp)?;

    // A commonly available object is 0x1018 (Identity). Subindex 0 is the
    // number of entries; size 4 is just an example.
    // Configure the first slave (position 0). Replace IDs and SDO index with
    // your device.
    let mut req = {
        let mut cfg = master.configure_slave(
            SlaveAddr::ByPos(0),
            SlaveId {
                vendor_id: 0,
                product_code: 0,
            },
        )?;

        cfg.create_sdo_request(
            SdoIdx {
                idx: Idx::from(0x1018),
                sub_idx: SubIdx::from(0u8),
            },
            4,
        )?
    };

    master.activate()?;

    // Trigger the first read.
    req.read()?;

    let cycle_time = Duration::from_millis(10);
    loop {
        master.receive()?;
        master.domain(domain_idx).process()?;
        master.domain(domain_idx).queue()?;

        match req.state()? {
            ec::EC_REQUEST_SUCCESS => {
                println!("SDO data ({} bytes): {:02X?}", req.data().len(), req.data());
                req.read()?; // trigger next read
            }
            ec::EC_REQUEST_ERROR => {
                eprintln!("SDO request failed; retrying");
                req.read()?;
            }
            ec::EC_REQUEST_UNUSED | ec::EC_REQUEST_BUSY => {
                // No completed result yet.
            }
            _ => {
                // Future/unknown states.
            }
        }

        master.send()?;
        thread::sleep(cycle_time);
    }
}
