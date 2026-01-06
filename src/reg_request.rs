// Part of ethercat-rs. Copyright 2018-2022 by the authors.
// This work is dual-licensed under Apache 2.0 and MIT terms.

use crate::{ec, types::*};
use std::{fmt, io, os::fd::RawFd};

/// A handle for a kernel-side register request.
///
/// A `RegisterRequest` owns a fixed-size userspace buffer that is exchanged with
/// the kernel when a read completes or when a write is scheduled.
///
/// This is intended for low-frequency diagnostics/monitoring during realtime
/// operation. It is not a replacement for PDOs or normal master configuration.
pub struct RegisterRequest {
    fd: RawFd,
    config_index: SlaveConfigIdx,
    request_index: u32,
    buffer: Vec<u8>,
}

impl RegisterRequest {
    /// Construct a new `RegisterRequest` handle.
    ///
    /// Intended to be called from `SlaveConfig::create_reg_request` after the
    /// kernel request has been registered and a `request_index` has been
    /// returned.
    pub(crate) fn new(
        fd: RawFd,
        config_index: SlaveConfigIdx,
        request_index: u32,
        mem_size: usize,
    ) -> Self {
        Self {
            fd,
            config_index,
            request_index,
            buffer: vec![0u8; mem_size],
        }
    }

    /// Return the kernel-side identifier that uniquely identifies this request.
    pub fn request_index(&self) -> u32 {
        self.request_index
    }

    /// Access the request's internal data buffer.
    ///
    /// After a successful read operation, this slice contains the data returned
    /// by the slave. Before scheduling a write, fill this buffer with the bytes
    /// you want to write.
    pub fn data(&self) -> &[u8] {
        &self.buffer
    }

    /// Mutable access to the request's internal data buffer.
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.buffer
    }

    /// Total size of the internal buffer in bytes.
    pub fn data_capacity(&self) -> usize {
        self.buffer.len()
    }

    /// Query the kernel for the current state of this register request.
    ///
    /// If the kernel indicates that a fresh read result is available, this
    /// method automatically fetches the data into the internal buffer.
    ///
    /// The returned value corresponds to `ec_request_state_t` in the C API.
    pub fn state(&mut self) -> Result<ec::ec_request_state_t> {
        let mut data = ec::ec_ioctl_reg_request_t::default();
        data.config_index = self.config_index;
        data.request_index = self.request_index;

        let res = unsafe { ec::ioctl::REG_REQUEST_STATE(self.fd, &mut data) };
        if res < 0 {
            return Err(Error::Io(io::Error::last_os_error()));
        }

        if data.new_data != 0 {
            let mut io = ec::ec_ioctl_reg_request_t::default();
            io.config_index = self.config_index;
            io.request_index = self.request_index;
            io.mem_size = self.buffer.len();
            io.data = self.buffer.as_mut_ptr();

            let res = unsafe { ec::ioctl::REG_REQUEST_DATA(self.fd, &mut io) };
            if res < 0 {
                return Err(Error::Io(io::Error::last_os_error()));
            }
        }

        Ok(data.state)
    }

    /// Schedule a register read operation.
    ///
    /// `size` is limited to the buffer size chosen at request creation.
    pub fn read(&mut self, address: u16, size: usize) -> Result<()> {
        let mut data = ec::ec_ioctl_reg_request_t::default();
        data.config_index = self.config_index;
        data.request_index = self.request_index;
        data.address = address;
        data.transfer_size = size.min(self.buffer.len());

        let res = unsafe { ec::ioctl::REG_REQUEST_READ(self.fd, &mut data) };
        if res < 0 {
            return Err(Error::Io(io::Error::last_os_error()));
        }
        Ok(())
    }

    /// Schedule a register write operation.
    ///
    /// The kernel copies `size` bytes from this request's internal buffer and
    /// writes them to `address`. `size` is limited to the buffer size chosen at
    /// request creation.
    pub fn write(&mut self, address: u16, size: usize) -> Result<()> {
        let mut data = ec::ec_ioctl_reg_request_t::default();
        data.config_index = self.config_index;
        data.request_index = self.request_index;
        data.address = address;
        data.transfer_size = size.min(self.buffer.len());
        data.data = self.buffer.as_mut_ptr();

        let res = unsafe { ec::ioctl::REG_REQUEST_WRITE(self.fd, &mut data) };
        if res < 0 {
            return Err(Error::Io(io::Error::last_os_error()));
        }
        Ok(())
    }
}

impl fmt::Debug for RegisterRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RegisterRequest")
            .field("config_index", &self.config_index)
            .field("request_index", &self.request_index)
            .field("buffer_len", &self.buffer.len())
            .finish()
    }
}
