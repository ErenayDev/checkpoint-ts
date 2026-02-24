#[cfg(windows)]
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

const RPC_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy)]
#[repr(u32)]
enum OpCode {
    Handshake = 0,
    Frame = 1,
    Close = 2,
    #[expect(dead_code)]
    Ping = 3,
    #[expect(dead_code)]
    Pong = 4,
}

#[derive(Debug, Default)]
pub struct Activity {
    pub state: Option<String>,
    pub details: Option<String>,
    pub large_image: Option<String>,
    pub large_text: Option<String>,
    pub small_image: Option<String>,
    pub small_text: Option<String>,
    pub start_timestamp: Option<u64>,
    pub buttons: Option<Vec<Button>>,
}

#[derive(Debug)]
pub struct Button {
    pub label: String,
    pub url: String,
}

impl Activity {
    pub fn new() -> Self {
        Self::default()
    }

    fn to_json(&self) -> serde_json::Value {
        let mut activity = serde_json::Map::new();

        if let Some(ref state) = self.state {
            activity.insert("state".into(), serde_json::Value::String(state.clone()));
        }
        if let Some(ref details) = self.details {
            activity.insert("details".into(), serde_json::Value::String(details.clone()));
        }

        let mut assets = serde_json::Map::new();
        let mut has_assets = false;

        if let Some(ref key) = self.large_image {
            assets.insert("large_image".into(), serde_json::Value::String(key.clone()));
            has_assets = true;
        }
        if let Some(ref text) = self.large_text {
            assets.insert("large_text".into(), serde_json::Value::String(text.clone()));
            has_assets = true;
        }
        if let Some(ref key) = self.small_image {
            assets.insert("small_image".into(), serde_json::Value::String(key.clone()));
            has_assets = true;
        }
        if let Some(ref text) = self.small_text {
            assets.insert("small_text".into(), serde_json::Value::String(text.clone()));
            has_assets = true;
        }

        if has_assets {
            activity.insert("assets".into(), serde_json::Value::Object(assets));
        }

        if let Some(ts) = self.start_timestamp {
            let timestamps = serde_json::json!({ "start": ts });
            activity.insert("timestamps".into(), timestamps);
        }

        if let Some(ref buttons) = self.buttons {
            let button_array: Vec<serde_json::Value> = buttons
                .iter()
                .map(|btn| {
                    serde_json::json!({
                        "label": btn.label,
                        "url": btn.url
                    })
                })
                .collect();
            activity.insert("buttons".into(), serde_json::Value::Array(button_array));
        }

        serde_json::Value::Object(activity)
    }
}

enum Connection {
    #[cfg(unix)]
    Unix(UnixStream),
    #[cfg(windows)]
    Pipe(std::fs::File),
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(unix)]
            Connection::Unix(_) => f.write_str("Connection::Unix(..)"),
            #[cfg(windows)]
            Connection::Pipe(_) => f.write_str("Connection::Pipe(..)"),
        }
    }
}

impl Connection {
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        match self {
            #[cfg(unix)]
            Connection::Unix(stream) => stream.write_all(buf),
            #[cfg(windows)]
            Connection::Pipe(file) => file.write_all(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            #[cfg(unix)]
            Connection::Unix(stream) => stream.flush(),
            #[cfg(windows)]
            Connection::Pipe(file) => file.flush(),
        }
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        match self {
            #[cfg(unix)]
            Connection::Unix(stream) => stream.read_exact(buf),
            #[cfg(windows)]
            Connection::Pipe(file) => file.read_exact(buf),
        }
    }
}

pub struct DiscordRpc {
    client_id: String,
    connection: Connection,
    pid: u32,
}

impl std::fmt::Debug for DiscordRpc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscordRpc")
            .field("client_id", &self.client_id)
            .field("pid", &self.pid)
            .finish_non_exhaustive()
    }
}

impl DiscordRpc {
    pub fn connect(client_id: &str) -> io::Result<Self> {
        let connection = Self::open_connection()?;
        let mut rpc = Self {
            client_id: client_id.to_string(),
            connection,
            pid: std::process::id(),
        };
        rpc.handshake()?;
        Ok(rpc)
    }

    #[cfg(unix)]
    fn open_connection() -> io::Result<Connection> {
        let candidates = Self::socket_paths();
        for path in &candidates {
            if let Ok(stream) = UnixStream::connect(path) {
                return Ok(Connection::Unix(stream));
            }
        }
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "discord ipc socket not found",
        ))
    }

    #[cfg(unix)]
    fn socket_paths() -> Vec<PathBuf> {
        let dirs: Vec<String> = [
            std::env::var("XDG_RUNTIME_DIR").ok(),
            std::env::var("TMPDIR").ok(),
            std::env::var("TMP").ok(),
            std::env::var("TEMP").ok(),
            Some("/tmp".into()),
        ]
        .into_iter()
        .flatten()
        .collect();

        let mut paths = Vec::new();
        for dir in dirs {
            for i in 0..10 {
                paths.push(PathBuf::from(&dir).join(format!("discord-ipc-{i}")));
                let snap_path = PathBuf::from(&dir)
                    .join("snap.discord")
                    .join(format!("discord-ipc-{i}"));
                paths.push(snap_path);
                let flatpak_path = PathBuf::from(&dir)
                    .join("app")
                    .join("com.discordapp.Discord")
                    .join(format!("discord-ipc-{i}"));
                paths.push(flatpak_path);
            }
        }
        paths
    }

    #[cfg(windows)]
    fn open_connection() -> io::Result<Connection> {
        for i in 0..10 {
            let pipe_name = format!(r"\\.\pipe\discord-ipc-{i}");
            let file = OpenOptions::new().read(true).write(true).open(&pipe_name);
            if let Ok(f) = file {
                return Ok(Connection::Pipe(f));
            }
        }
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "discord ipc pipe not found",
        ))
    }

    fn handshake(&mut self) -> io::Result<()> {
        let payload = serde_json::json!({
            "v": RPC_VERSION,
            "client_id": self.client_id,
        });
        self.send(OpCode::Handshake, &payload)?;
        let (_op, _response) = self.recv()?;
        Ok(())
    }

    pub fn set_activity(&mut self, activity: &Activity) -> io::Result<()> {
        let payload = serde_json::json!({
            "cmd": "SET_ACTIVITY",
            "args": {
                "pid": self.pid,
                "activity": activity.to_json(),
            },
            "nonce": Self::nonce(),
        });
        self.send(OpCode::Frame, &payload)?;
        let _ = self.recv()?;
        Ok(())
    }

    pub fn clear_activity(&mut self) -> io::Result<()> {
        let payload = serde_json::json!({
            "cmd": "SET_ACTIVITY",
            "args": {
                "pid": self.pid,
            },
            "nonce": Self::nonce(),
        });
        self.send(OpCode::Frame, &payload)?;
        let _ = self.recv()?;
        Ok(())
    }

    fn send(&mut self, opcode: OpCode, payload: &serde_json::Value) -> io::Result<()> {
        let json_bytes = serde_json::to_vec(payload)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut header = [0u8; 8];
        header[..4].copy_from_slice(&(opcode as u32).to_le_bytes());
        header[4..8].copy_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        self.connection.write_all(&header)?;
        self.connection.write_all(&json_bytes)?;
        self.connection.flush()
    }

    fn recv(&mut self) -> io::Result<(u32, serde_json::Value)> {
        let mut header = [0u8; 8];
        self.connection.read_exact(&mut header)?;
        let opcode = u32::from_le_bytes(header[..4].try_into().unwrap());
        let length = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
        let mut buf = vec![0u8; length];
        self.connection.read_exact(&mut buf)?;
        let value: serde_json::Value = serde_json::from_slice(&buf)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        if opcode == OpCode::Close as u32 {
            let msg = value["message"]
                .as_str()
                .unwrap_or("connection closed by discord");
            return Err(io::Error::new(io::ErrorKind::ConnectionReset, msg));
        }

        Ok((opcode, value))
    }

    fn nonce() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        format!("{}{}", ts.as_nanos(), std::process::id())
    }
}

impl Drop for DiscordRpc {
    fn drop(&mut self) {
        let _ = self.clear_activity();
        let payload = serde_json::json!({});
        let _ = self.send(OpCode::Close, &payload);
    }
}

#[derive(Debug)]
pub struct RpcSession {
    rpc: Option<DiscordRpc>,
    pub activity: Activity,
}

impl RpcSession {
    pub fn new(rpc: Option<DiscordRpc>, activity: Activity) -> Self {
        Self { rpc, activity }
    }

    pub fn update(&mut self) {
        if let Some(ref mut rpc) = self.rpc {
            let _ = rpc.set_activity(&self.activity);
        }
    }

    pub fn connected(&self) -> bool {
        self.rpc.is_some()
    }
}
