// SDO request wrapper extracted from `master.rs`.
use crate::{ec, types::*, Master};
use std::{fmt, io};

/// A handle for a kernel-side SDO request.
pub struct SdoRequest<'m> {
    master: &'m Master,
    config_index: SlaveConfigIdx,
    request_index: u32,
    sdo_index: SdoIdx,
    buffer: Vec<u8>,
    data_size: usize,
}

impl<'m> SdoRequest<'m> {
    /// Construct a new `SdoRequest` handle. Intended to be called from
    /// `SlaveConfig::create_sdo_request` after the kernel request has been
    /// created and returned a `request_index` and buffer size.
    pub(crate) fn new(
        master: &'m Master,
        config_index: SlaveConfigIdx,
        request_index: u32,
        sdo_index: SdoIdx,
        size: usize,
    ) -> Self {
        Self {
            master,
            config_index,
            request_index,
            sdo_index,
            buffer: vec![0u8; size],
            data_size: size,
        }
    }

    /// Return the SDO index/subindex configured for this request.
    pub fn index(&self) -> SdoIdx {
        self.sdo_index
    }

    /// Set the SDO index and subindex for this request.
    ///
    /// This maps to `ecrt_sdo_request_index()` in the C API.
    pub fn set_index(&mut self, index: SdoIdx) -> Result<()> {
        let mut data = ec::ec_ioctl_sdo_request_t::default();
        data.config_index = self.config_index;
        data.request_index = self.request_index;
        data.sdo_index = u16::from(index.idx);
        data.sdo_subindex = u8::from(index.sub_idx);

        let res = unsafe { ec::ioctl::SDO_REQUEST_INDEX(self.master.fd(), &mut data) };
        if res < 0 {
            return Err(Error::Io(io::Error::last_os_error()));
        }
        self.sdo_index = index;
        Ok(())
    }

    /// Return the kernel-side identifier that uniquely identifies this request.
    pub fn request_index(&self) -> u32 {
        self.request_index
    }

    /// Return a read-only view of the currently valid data for this request.
    pub fn data(&self) -> &[u8] {
        &self.buffer[..self.data_size]
    }

    /// Return a mutable view of the currently valid data buffer.
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.buffer[..self.data_size]
    }

    /// Return the total capacity of the internally allocated buffer.
    pub fn data_capacity(&self) -> usize {
        self.buffer.len()
    }

    /// Set the logical size of the request data.
    pub fn set_data_size(&mut self, len: usize) -> Result<()> {
        if len > self.buffer.len() {
            return Err(Error::RequestFailed);
        }
        self.data_size = len;
        Ok(())
    }

    /// Return the current logical size of the request data.
    ///
    /// This maps to `ecrt_sdo_request_data_size()` in the C API.
    pub fn data_size(&self) -> usize {
        self.data_size
    }

    /// Set the timeout (in microseconds) for this request in the kernel.
    ///
    /// This maps to `ecrt_sdo_request_timeout()` in the C API.
    pub fn set_timeout(&mut self, timeout_us: u32) -> Result<()> {
        let mut data = ec::ec_ioctl_sdo_request_t::default();
        data.config_index = self.config_index;
        data.request_index = self.request_index;
        data.timeout = timeout_us;

        let res = unsafe { ec::ioctl::SDO_REQUEST_TIMEOUT(self.master.fd(), &mut data) };
        if res < 0 {
            return Err(Error::Io(io::Error::last_os_error()));
        }
        Ok(())
    }

    /// Query the kernel for the current state of this SDO request.
    ///
    /// If new data is available, the kernel will provide the data size via
    /// `SDO_REQUEST_STATE`. In that case this method reads the data via
    /// `SDO_REQUEST_DATA` and fills the internal buffer (and updates
    /// `data_size`). The returned value is the kernel's request state.
    pub fn state(&mut self) -> Result<ec::ec_request_state_t> {
        let mut data = ec::ec_ioctl_sdo_request_t::default();
        data.config_index = self.config_index;
        data.request_index = self.request_index;

        let res = unsafe { ec::ioctl::SDO_REQUEST_STATE(self.master.fd(), &mut data) };
        if res < 0 {
            return Err(Error::Io(io::Error::last_os_error()));
        }

        if data.size != 0 {
            if self.buffer.len() < data.size {
                log::error!(
                    "Received {} bytes do not fit into SDO request buffer ({} bytes)",
                    data.size,
                    self.buffer.len()
                );
                return Ok(ec::EC_REQUEST_ERROR);
            }

            data.data = self.buffer.as_mut_ptr();
            let res = unsafe { ec::ioctl::SDO_REQUEST_DATA(self.master.fd(), &mut data) };
            if res < 0 {
                return Err(Error::Io(io::Error::last_os_error()));
            }
            self.data_size = data.size;
        }

        Ok(data.state)
    }

    /// Trigger a read operation for this request in the kernel.
    ///
    /// This maps to `ecrt_sdo_request_read()` in the C API.
    pub fn read(&mut self) -> Result<()> {
        let mut data = ec::ec_ioctl_sdo_request_t::default();
        data.config_index = self.config_index;
        data.request_index = self.request_index;

        let res = unsafe { ec::ioctl::SDO_REQUEST_READ(self.master.fd(), &mut data) };
        if res < 0 {
            return Err(Error::Io(io::Error::last_os_error()));
        }
        Ok(())
    }

    /// Trigger a write operation for this request in the kernel using the
    /// contents of the internal buffer up to the current `data_size`.
    ///
    /// This maps to `ecrt_sdo_request_write()` in the C API.
    pub fn write(&mut self) -> Result<()> {
        let mut data = ec::ec_ioctl_sdo_request_t::default();
        data.config_index = self.config_index;
        data.request_index = self.request_index;
        data.data = self.buffer.as_mut_ptr();
        data.size = self.data_size;

        let res = unsafe { ec::ioctl::SDO_REQUEST_WRITE(self.master.fd(), &mut data) };
        if res < 0 {
            return Err(Error::Io(io::Error::last_os_error()));
        }
        Ok(())
    }
}

impl fmt::Debug for SdoRequest<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SdoRequest")
            .field("config_index", &self.config_index)
            .field("request_index", &self.request_index)
            .field("sdo_index", &self.sdo_index)
            .field("data_size", &self.data_size)
            .field("buffer_len", &self.buffer.len())
            .finish()
    }
}
