use shared_memory::{Shmem, ShmemConf};
use std::fmt;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

pub const QUEUE_CAPACITY: usize = 16;
pub const MESSAGE_SIZE: usize = 64 * 1024;
const RING_BUFFER_HEADER_SIZE: usize = 32;
const SLOT_HEADER_SIZE: usize = 8;
const SLOT_SIZE: usize = MESSAGE_SIZE;
const SLOTS_TOTAL_SIZE: usize = SLOT_SIZE * QUEUE_CAPACITY;
const SINGLE_CHANNEL_SIZE: usize = RING_BUFFER_HEADER_SIZE + SLOTS_TOTAL_SIZE;
const NUM_CHANNELS: usize = 4;
const TOTAL_SIZE: usize = SINGLE_CHANNEL_SIZE * NUM_CHANNELS;

#[repr(C)]
struct RingBufferHeader {
    write_index: AtomicU32,
    read_index: AtomicU32,
    capacity: u32,
    message_size: u32,
    reserved: [u8; 16],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    RustToTsCommand = 0,
    TsToRustStatus = 1,
    TsToRustCheckpoint = 2,
    RustToTsCheckpointResponse = 3,
}

#[derive(Debug, Clone)]
pub enum RingBufferError {
    QueueFull,
    MessageTooLarge,
    InvalidChannel,
    SerializationError(String),
    SizeMismatch(usize, usize),
}

impl fmt::Display for RingBufferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RingBufferError::QueueFull => write!(f, "ring buffer queue is full"),
            RingBufferError::MessageTooLarge => write!(f, "message exceeds maximum size"),
            RingBufferError::InvalidChannel => write!(f, "invalid channel index"),
            RingBufferError::SerializationError(msg) => write!(f, "serialization error: {}", msg),
            RingBufferError::SizeMismatch(actual, expected) => write!(
                f,
                "shared memory size mismatch: got {}, expected {}",
                actual, expected
            ),
        }
    }
}

impl std::error::Error for RingBufferError {}

type LogCallback = Arc<Mutex<Option<Box<dyn Fn(String) + Send + 'static>>>>;

pub struct SharedMemoryBridge {
    shmem: Shmem,
    request_id_counter: AtomicU64,
    log_callback: LogCallback,
}

impl SharedMemoryBridge {
    pub fn create() -> Result<Self, shared_memory::ShmemError> {
        let shmem = ShmemConf::new().size(TOTAL_SIZE).create()?;

        for channel_idx in 0..NUM_CHANNELS {
            let offset = channel_idx * SINGLE_CHANNEL_SIZE;
            let header = unsafe { &*(shmem.as_ptr().add(offset) as *const RingBufferHeader) };

            header.write_index.store(0, Ordering::SeqCst);
            header.read_index.store(0, Ordering::SeqCst);

            let capacity_ptr = unsafe { shmem.as_ptr().add(offset + 8) as *mut u32 };
            unsafe { *capacity_ptr = QUEUE_CAPACITY as u32 };

            let message_size_ptr = unsafe { shmem.as_ptr().add(offset + 12) as *mut u32 };
            unsafe { *message_size_ptr = MESSAGE_SIZE as u32 };

            for slot_idx in 0..QUEUE_CAPACITY {
                let slot_offset = offset + RING_BUFFER_HEADER_SIZE + (slot_idx * SLOT_SIZE);
                let ready_flag = unsafe { &*(shmem.as_ptr().add(slot_offset) as *const AtomicU32) };
                ready_flag.store(0, Ordering::SeqCst);
            }
        }

        Ok(Self {
            shmem,
            request_id_counter: AtomicU64::new(1),
            log_callback: Arc::new(Mutex::new(None)),
        })
    }

    pub fn open(os_id: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let shmem = ShmemConf::new().os_id(os_id).open()?;

        if shmem.len() != TOTAL_SIZE {
            return Err(Box::new(RingBufferError::SizeMismatch(
                shmem.len(),
                TOTAL_SIZE,
            )));
        }
        Ok(Self {
            shmem,
            request_id_counter: AtomicU64::new(1),
            log_callback: Arc::new(Mutex::new(None)),
        })
    }

    pub fn set_log_callback<F>(&self, callback: F)
    where
        F: Fn(String) + Send + 'static,
    {
        if let Ok(mut cb) = self.log_callback.lock() {
            *cb = Some(Box::new(callback));
        }
    }

    fn log(&self, message: String) {
        if std::env::var("CHECKPOINT_VERBOSE").as_deref() != Ok("1") {
            return;
        }

        if let Ok(cb) = self.log_callback.lock() {
            if let Some(ref callback) = *cb {
                callback(message);
            }
        }
    }

    pub fn os_id(&self) -> &str {
        self.shmem.get_os_id()
    }

    pub fn next_request_id(&self) -> u64 {
        self.request_id_counter.fetch_add(1, Ordering::SeqCst)
    }

    fn get_header(&self, channel: Channel) -> &RingBufferHeader {
        let offset = (channel as usize) * SINGLE_CHANNEL_SIZE;
        unsafe { &*(self.shmem.as_ptr().add(offset) as *const RingBufferHeader) }
    }

    fn get_slot_ptr(&self, channel: Channel, slot_index: usize) -> *mut u8 {
        let channel_offset = (channel as usize) * SINGLE_CHANNEL_SIZE;
        let slot_offset = RING_BUFFER_HEADER_SIZE + (slot_index * SLOT_SIZE);
        unsafe { self.shmem.as_ptr().add(channel_offset + slot_offset) }
    }

    fn write_to_slot(
        &self,
        channel: Channel,
        slot_index: usize,
        data: &[u8],
    ) -> Result<(), RingBufferError> {
        if data.len() > MESSAGE_SIZE - SLOT_HEADER_SIZE {
            return Err(RingBufferError::MessageTooLarge);
        }

        let slot_ptr = self.get_slot_ptr(channel, slot_index);

        let ready_flag = unsafe { &*(slot_ptr as *const AtomicU32) };
        ready_flag.store(0, Ordering::SeqCst);

        let length_ptr = unsafe { slot_ptr.add(4) as *mut u32 };
        unsafe { *length_ptr = data.len() as u32 };

        let data_ptr = unsafe { slot_ptr.add(SLOT_HEADER_SIZE) };
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), data_ptr, data.len());
        }

        ready_flag.store(1, Ordering::SeqCst);

        Ok(())
    }

    fn read_from_slot(&self, channel: Channel, slot_index: usize) -> Option<Vec<u8>> {
        let slot_ptr = self.get_slot_ptr(channel, slot_index);

        let ready_flag = unsafe { &*(slot_ptr as *const AtomicU32) };

        let mut spin_count = 0;
        loop {
            if ready_flag.load(Ordering::SeqCst) == 1 {
                break;
            }
            spin_count += 1;
            if spin_count > 100000 {
                self.log(format!(
                    "[SLOT-READ] Timeout waiting for slot {}",
                    slot_index
                ));
                return None;
            }
            std::hint::spin_loop();
        }

        let length_ptr = unsafe { slot_ptr.add(4) as *const u32 };
        let data_length = unsafe { *length_ptr } as usize;

        if data_length > MESSAGE_SIZE - SLOT_HEADER_SIZE {
            return None;
        }

        let data_ptr = unsafe { slot_ptr.add(SLOT_HEADER_SIZE) };
        let mut buffer = vec![0u8; data_length];
        unsafe {
            std::ptr::copy_nonoverlapping(data_ptr, buffer.as_mut_ptr(), data_length);
        }

        Some(buffer)
    }

    fn enqueue_spsc(&self, channel: Channel, data: &[u8]) -> Result<(), RingBufferError> {
        let header = self.get_header(channel);

        let current_write_index = header.write_index.load(Ordering::SeqCst);
        let current_read_index = header.read_index.load(Ordering::SeqCst);

        let next_write_index = (current_write_index + 1) % QUEUE_CAPACITY as u32;

        if next_write_index == current_read_index {
            self.log(format!(
                "[SPSC] Queue full! write={}, read={}",
                current_write_index, current_read_index
            ));
            return Err(RingBufferError::QueueFull);
        }

        self.write_to_slot(channel, current_write_index as usize, data)?;

        header.write_index.store(next_write_index, Ordering::SeqCst);

        Ok(())
    }

    fn enqueue_mpsc(&self, channel: Channel, data: &[u8]) -> Result<(), RingBufferError> {
        if data.len() > MESSAGE_SIZE - SLOT_HEADER_SIZE {
            return Err(RingBufferError::MessageTooLarge);
        }

        let header = self.get_header(channel);

        loop {
            let current_write_index = header.write_index.load(Ordering::SeqCst);
            let current_read_index = header.read_index.load(Ordering::SeqCst);

            let next_write_index = (current_write_index + 1) % QUEUE_CAPACITY as u32;

            if next_write_index == current_read_index {
                self.log(format!(
                    "[MPSC] Queue full! write={}, read={}",
                    current_write_index, current_read_index
                ));
                return Err(RingBufferError::QueueFull);
            }

            match header.write_index.compare_exchange(
                current_write_index,
                next_write_index,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => {
                    self.log(format!("[MPSC] CAS won: slot[{}]", current_write_index));

                    self.write_to_slot(channel, current_write_index as usize, data)
                        .expect("write_to_slot failed after size validation");

                    return Ok(());
                }
                Err(_) => {
                    std::hint::spin_loop();
                    continue;
                }
            }
        }
    }

    // TODO: Reduce diagnostic logging volume in shared memory operations.
    // Per-slot dequeue/enqueue logs are useful for debugging the IPC layer
    // but produce excessive output during normal operation. Introduce a
    // log level mechanism (e.g., trace vs. debug) and gate these calls
    // behind a verbose flag or compile-time feature.
    fn dequeue(&self, channel: Channel) -> Option<Vec<u8>> {
        let header = self.get_header(channel);

        let current_read_index = header.read_index.load(Ordering::SeqCst);
        let current_write_index = header.write_index.load(Ordering::SeqCst);

        if current_read_index == current_write_index {
            return None;
        }

        self.log(format!("[DEQUEUE] Reading slot[{}]", current_read_index));

        let data = self.read_from_slot(channel, current_read_index as usize)?;

        let next_read_index = (current_read_index + 1) % QUEUE_CAPACITY as u32;
        header.read_index.store(next_read_index, Ordering::SeqCst);

        self.log(format!(
            "[DEQUEUE] Slot[{}] read complete",
            current_read_index
        ));

        Some(data)
    }

    fn wait_dequeue(&self, channel: Channel, timeout_ms: u64) -> Option<Vec<u8>> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);
        let spin_threshold = std::time::Duration::from_micros(100);

        loop {
            if let Some(data) = self.dequeue(channel) {
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

    pub fn send_command(&self, data: &[u8]) -> Result<(), RingBufferError> {
        self.enqueue_spsc(Channel::RustToTsCommand, data)
    }

    pub fn send_command_json<T: serde::Serialize>(&self, value: &T) -> Result<(), RingBufferError> {
        let json = serde_json::to_vec(value)
            .map_err(|e| RingBufferError::SerializationError(e.to_string()))?;
        self.send_command(&json)
    }

    pub fn receive_status(&self) -> Option<Vec<u8>> {
        self.dequeue(Channel::TsToRustStatus)
    }

    pub fn receive_status_json<T: serde::de::DeserializeOwned>(&self) -> Option<T> {
        let data = self.receive_status()?;
        serde_json::from_slice(&data).ok()
    }

    pub fn wait_receive_status(&self, timeout_ms: u64) -> Option<Vec<u8>> {
        self.wait_dequeue(Channel::TsToRustStatus, timeout_ms)
    }

    pub fn wait_receive_status_json<T: serde::de::DeserializeOwned>(
        &self,
        timeout_ms: u64,
    ) -> Option<T> {
        let data = self.wait_receive_status(timeout_ms)?;
        serde_json::from_slice(&data).ok()
    }

    pub fn receive_checkpoint(&self) -> Option<Vec<u8>> {
        self.dequeue(Channel::TsToRustCheckpoint)
    }

    pub fn receive_checkpoint_json<T: serde::de::DeserializeOwned>(&self) -> Option<T> {
        let data = self.receive_checkpoint()?;
        serde_json::from_slice(&data).ok()
    }

    pub fn wait_receive_checkpoint(&self, timeout_ms: u64) -> Option<Vec<u8>> {
        self.wait_dequeue(Channel::TsToRustCheckpoint, timeout_ms)
    }

    pub fn wait_receive_checkpoint_json<T: serde::de::DeserializeOwned>(
        &self,
        timeout_ms: u64,
    ) -> Option<T> {
        let data = self.wait_receive_checkpoint(timeout_ms)?;
        serde_json::from_slice(&data).ok()
    }

    pub fn send_checkpoint_response(&self, data: &[u8]) -> Result<(), RingBufferError> {
        self.enqueue_spsc(Channel::RustToTsCheckpointResponse, data)
    }

    pub fn send_checkpoint_response_json<T: serde::Serialize>(
        &self,
        value: &T,
    ) -> Result<(), RingBufferError> {
        let json = serde_json::to_vec(value)
            .map_err(|e| RingBufferError::SerializationError(e.to_string()))?;
        self.send_checkpoint_response(&json)
    }

    pub fn send_checkpoint(&self, data: &[u8]) -> Result<(), RingBufferError> {
        self.enqueue_mpsc(Channel::TsToRustCheckpoint, data)
    }

    pub fn send_checkpoint_json<T: serde::Serialize>(
        &self,
        value: &T,
    ) -> Result<(), RingBufferError> {
        let json = serde_json::to_vec(value)
            .map_err(|e| RingBufferError::SerializationError(e.to_string()))?;
        self.send_checkpoint(&json)
    }

    pub fn debug_queue_state(&self, channel: Channel) {
        let header = self.get_header(channel);
        let write_idx = header.write_index.load(Ordering::SeqCst);
        let read_idx = header.read_index.load(Ordering::SeqCst);

        let count = if write_idx >= read_idx {
            write_idx - read_idx
        } else {
            QUEUE_CAPACITY as u32 - read_idx + write_idx
        };

        self.log(format!(
            "[DEBUG] Channel {:?}: write_idx={}, read_idx={}, messages_queued={}/{}",
            channel, write_idx, read_idx, count, QUEUE_CAPACITY
        ));
    }

    pub fn debug_all_queues(&self) {
        self.log("[DEBUG] ===== Queue State =====".to_string());
        self.debug_queue_state(Channel::RustToTsCommand);
        self.debug_queue_state(Channel::TsToRustStatus);
        self.debug_queue_state(Channel::TsToRustCheckpoint);
        self.debug_queue_state(Channel::RustToTsCheckpointResponse);
        self.log("[DEBUG] =======================".to_string());
    }
}
