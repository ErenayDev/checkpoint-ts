use shared_memory::{Shmem, ShmemConf};
use std::sync::atomic::{AtomicU32, Ordering};

const HEADER_SIZE: usize = 8;
const BUFFER_SIZE: usize = 1024 * 1024;
const SINGLE_CHANNEL_SIZE: usize = HEADER_SIZE + BUFFER_SIZE;
const TOTAL_SIZE: usize = SINGLE_CHANNEL_SIZE * 2;

#[repr(C)]
struct ChannelHeader {
    ready_flag: AtomicU32,
    data_len: AtomicU32,
}

pub struct SharedMemoryBridge {
    shmem: Shmem,
}

impl SharedMemoryBridge {
    pub fn create() -> Result<Self, shared_memory::ShmemError> {
        let shmem = ShmemConf::new().size(TOTAL_SIZE).create()?;

        let rust_to_ts = unsafe { &*(shmem.as_ptr() as *const ChannelHeader) };
        let ts_to_rust =
            unsafe { &*(shmem.as_ptr().add(SINGLE_CHANNEL_SIZE) as *const ChannelHeader) };

        rust_to_ts.ready_flag.store(0, Ordering::SeqCst);
        rust_to_ts.data_len.store(0, Ordering::SeqCst);
        ts_to_rust.ready_flag.store(0, Ordering::SeqCst);
        ts_to_rust.data_len.store(0, Ordering::SeqCst);

        Ok(Self { shmem })
    }

    pub fn open(os_id: &str) -> Result<Self, shared_memory::ShmemError> {
        let shmem = ShmemConf::new().os_id(os_id).open()?;
        Ok(Self { shmem })
    }

    pub fn os_id(&self) -> &str {
        self.shmem.get_os_id()
    }

    pub fn send(&self, data: &[u8]) -> Result<(), &'static str> {
        if data.len() > BUFFER_SIZE {
            return Err("data too large");
        }

        let header = unsafe { &*(self.shmem.as_ptr() as *const ChannelHeader) };
        let buffer = unsafe {
            std::slice::from_raw_parts_mut(
                self.shmem.as_ptr().add(HEADER_SIZE), // as *mut u8
                BUFFER_SIZE,
            )
        };

        buffer[..data.len()].copy_from_slice(data);
        header.data_len.store(data.len() as u32, Ordering::SeqCst);
        header.ready_flag.store(1, Ordering::SeqCst);

        Ok(())
    }

    pub fn send_json<T: serde::Serialize>(
        &self,
        value: &T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_vec(value)?;
        self.send(&json).map_err(|e| e.into())
    }

    pub fn receive(&self) -> Option<Vec<u8>> {
        let header =
            unsafe { &*(self.shmem.as_ptr().add(SINGLE_CHANNEL_SIZE) as *const ChannelHeader) };

        if header.ready_flag.load(Ordering::SeqCst) == 0 {
            return None;
        }

        let len = header.data_len.load(Ordering::SeqCst) as usize;
        if len > BUFFER_SIZE {
            header.ready_flag.store(0, Ordering::SeqCst);
            return None; // or return an error
        }
        let buffer = unsafe {
            std::slice::from_raw_parts(
                self.shmem.as_ptr().add(SINGLE_CHANNEL_SIZE + HEADER_SIZE),
                len,
            )
        };

        let data = buffer.to_vec();
        header.ready_flag.store(0, Ordering::SeqCst);

        Some(data)
    }

    pub fn receive_json<T: serde::de::DeserializeOwned>(&self) -> Option<T> {
        let data = self.receive()?;
        serde_json::from_slice(&data).ok()
    }

    pub fn wait_receive(&self, timeout_ms: u64) -> Option<Vec<u8>> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);
        let spin_threshold = std::time::Duration::from_micros(100);

        loop {
            if let Some(data) = self.receive() {
                return Some(data);
            }

            if start.elapsed() > timeout {
                return None;
            }

            if start.elapsed() < spin_threshold {
                std::hint::spin_loop();
            } else {
                std::thread::sleep(std::time::Duration::from_micros(100));
            }
        }
    }

    pub fn wait_receive_json<T: serde::de::DeserializeOwned>(&self, timeout_ms: u64) -> Option<T> {
        let data = self.wait_receive(timeout_ms)?;
        serde_json::from_slice(&data).ok()
    }
}
