use checkpoint_shared::SharedMemoryBridge;
use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Stdio};

pub struct IpcBridge {
    shm: SharedMemoryBridge,
    child: Option<Child>,
    buffered_status_messages: Vec<serde_json::Value>,
}

impl IpcBridge {
    pub fn spawn_runtime(app_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let shm = SharedMemoryBridge::create()?;
        let shm_id = shm.os_id().to_string();

        let app_path_buf = Path::new(app_path);
        let runtime_dir = app_path_buf
            .parent()
            .and_then(|p| p.parent())
            .ok_or("Cannot find runtime directory")?
            .join("runtime");

        let runtime_index = runtime_dir.join("index.ts");
        if !runtime_index.exists() {
            return Err(format!("Runtime not found at {:?}", runtime_index).into());
        }

        let abs_app_path = std::fs::canonicalize(app_path_buf)?;
        let log_path = runtime_dir.join("runtime.log");
        let error_path = runtime_dir.join("runtime.error.log");

        let mut log_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&log_path)?;
        writeln!(log_file, "[RUST] Starting runtime with SHM: {}", shm_id)?;
        writeln!(log_file, "[RUST] App path: {}", abs_app_path.display())?;
        drop(log_file);

        let log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        let error_file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&error_path)?;

        let child = Command::new("bun")
            .arg("run")
            .arg("index.ts")
            .env("CHECKPOINT_SHM_ID", &shm_id)
            .env(
                "CHECKPOINT_APP_PATH",
                abs_app_path.to_string_lossy().as_ref(),
            )
            .current_dir(&runtime_dir)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log_file))
            .stderr(Stdio::from(error_file))
            .spawn()?;

        std::thread::sleep(std::time::Duration::from_millis(500));

        let mut attempts = 0;
        let mut ready = false;
        let mut buffered_status_messages = Vec::new();

        while attempts < 20 && !ready {
            attempts += 1;
            if let Some(msg) = shm.wait_receive_status_json::<serde_json::Value>(500) {
                if msg.get("type").and_then(|v| v.as_str()) == Some("runtime_ready") {
                    ready = true;
                }
                buffered_status_messages.push(msg);
            }
        }

        if !ready {
            let error_content = std::fs::read_to_string(&error_path)
                .unwrap_or_else(|_| "Could not read error log".to_string());
            return Err(format!("Runtime failed to start. Error log:\n{}", error_content).into());
        }

        Ok(Self {
            shm,
            child: Some(child),
            buffered_status_messages,
        })
    }

    pub fn set_log_callback<F>(&mut self, callback: F)
    where
        F: Fn(String) + Send + 'static,
    {
        self.shm.set_log_callback(callback);
    }

    pub fn load_app(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(self.shm.send_command_json(&serde_json::json!({
            "type": "load_app"
        }))?)
    }

    pub fn send_command_json<T: serde::Serialize>(
        &self,
        value: &T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(self.shm.send_command_json(value)?)
    }

    pub fn receive_status_json<T: serde::de::DeserializeOwned>(
        &mut self,
        timeout_ms: u64,
    ) -> Option<T> {
        if !self.buffered_status_messages.is_empty() {
            let msg = self.buffered_status_messages.remove(0);
            return serde_json::from_value(msg).ok();
        }

        let result_value = self
            .shm
            .wait_receive_status_json::<serde_json::Value>(timeout_ms);

        result_value.and_then(|v| serde_json::from_value(v).ok())
    }

    pub fn receive_checkpoint_json<T: serde::de::DeserializeOwned>(
        &self,
        timeout_ms: u64,
    ) -> Option<T> {
        let result = self
            .shm
            .wait_receive_checkpoint_json::<serde_json::Value>(timeout_ms);

        result.and_then(|v| serde_json::from_value(v).ok())
    }

    pub fn send_checkpoint_response_json<T: serde::Serialize>(
        &self,
        value: &T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(self.shm.send_checkpoint_response_json(value)?)
    }

    pub fn debug_queue_state(&self) {
        self.shm.debug_all_queues();
    }

    pub fn shutdown(&mut self) {
        let _ = self.send_command_json(&serde_json::json!({ "type": "shutdown" }));
        if let Some(ref mut child) = self.child {
            let _ = child.wait();
        }
    }
}

impl Drop for IpcBridge {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl std::fmt::Debug for IpcBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IpcBridge")
            .field("child", &self.child)
            .field(
                "buffered_status_count",
                &self.buffered_status_messages.len(),
            )
            .finish_non_exhaustive()
    }
}
