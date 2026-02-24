use checkpoint_shared::SharedMemoryBridge;
use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Stdio};

pub struct IpcBridge {
    shm: SharedMemoryBridge,
    child: Option<Child>,
    buffered_messages: Vec<serde_json::Value>,
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

        std::thread::sleep(std::time::Duration::from_millis(200));

        let mut attempts = 0;
        let mut ready = false;
        let mut buffered_messages = Vec::new();

        while attempts < 10 && !ready {
            if let Some(msg) = shm.wait_receive_json::<serde_json::Value>(500) {
                if msg.get("type").and_then(|v| v.as_str()) == Some("runtime_ready") {
                    ready = true;
                }
                buffered_messages.push(msg);
            } else {
                attempts += 1;
            }
        }

        if !ready {
            return Err("Runtime failed to start".into());
        }

        Ok(Self {
            shm,
            child: Some(child),
            buffered_messages,
        })
    }

    pub fn load_app(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.send_json(&serde_json::json!({
            "type": "load_app"
        }))
    }

    pub fn send_json<T: serde::Serialize>(
        &self,
        value: &T,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.shm.send_json(value)
    }

    pub fn receive_json<T: serde::de::DeserializeOwned>(&mut self, timeout_ms: u64) -> Option<T> {
        if !self.buffered_messages.is_empty() {
            let msg = self.buffered_messages.remove(0);
            return serde_json::from_value(msg).ok();
        }

        self.shm.wait_receive_json(timeout_ms)
    }

    pub fn shutdown(&mut self) {
        let _ = self.send_json(&serde_json::json!({ "type": "shutdown" }));
        if let Some(ref mut child) = self.child {
            let _ = child.wait();
        }
    }
}

impl Drop for IpcBridge {
    fn drop(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
        }
    }
}

impl std::fmt::Debug for IpcBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IpcBridge")
            .field("child", &self.child)
            .field("buffered_messages_count", &self.buffered_messages.len())
            .finish_non_exhaustive()
    }
}
