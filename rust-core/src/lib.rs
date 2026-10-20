use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
pub mod vault;
use std::fmt;
use std::io::{Read, Write};

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn SetConsoleCtrlHandler(
        handler: Option<unsafe extern "system" fn(u32) -> i32>,
        add: i32,
    ) -> i32;
}

/// Windows stores "ignore Ctrl+C" as an inheritable per-process flag. If whatever launched the
/// application had it set (some shells, CI runners and automation harnesses do), every terminal
/// child inherits it and Ctrl+C silently does nothing. Clearing it in this process before a child
/// is spawned gives each terminal normal Ctrl+C handling regardless of how the app was started.
#[cfg(windows)]
fn restore_default_ctrl_c_handling() {
    unsafe {
        SetConsoleCtrlHandler(None, 0);
    }
}

#[cfg(not(windows))]
fn restore_default_ctrl_c_handling() {}

pub struct TerminalProcess {
    master: Box<dyn MasterPty + Send>,
    reader: Option<Box<dyn Read + Send>>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl TerminalProcess {
    pub fn spawn(program: &str, args: &[&str]) -> anyhow::Result<Self> {
        Self::spawn_with_env(program, args, &std::collections::HashMap::new())
    }

    pub fn spawn_with_env(
        program: &str,
        args: &[&str],
        environment: &std::collections::HashMap<String, String>,
    ) -> anyhow::Result<Self> {
        Self::spawn_with_env_and_cwd(program, args, environment, None)
    }

    pub fn spawn_with_env_and_cwd(
        program: &str,
        args: &[&str],
        environment: &std::collections::HashMap<String, String>,
        cwd: Option<&str>,
    ) -> anyhow::Result<Self> {
        restore_default_ctrl_c_handling();
        let system = native_pty_system();
        let pair = system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let mut command = CommandBuilder::new(program);
        command.args(args.iter().copied());
        if let Some(cwd) = cwd.filter(|value| !value.trim().is_empty()) {
            command.cwd(cwd);
        }
        for (key, value) in environment {
            command.env(key, value);
        }
        let child = pair.slave.spawn_command(command)?;
        let master = pair.master;
        let reader = master.try_clone_reader()?;
        let writer = master.take_writer()?;
        Ok(Self {
            master,
            reader: Some(reader),
            writer,
            child,
        })
    }

    pub fn take_reader(&mut self) -> anyhow::Result<Box<dyn Read + Send>> {
        self.reader
            .take()
            .ok_or_else(|| anyhow::anyhow!("terminal reader already detached"))
    }

    pub fn read_to_end(&mut self) -> anyhow::Result<Vec<u8>> {
        let mut output = Vec::new();
        self.reader
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("terminal reader already detached"))?
            .read_to_end(&mut output)?;
        let _ = self.child.wait();
        Ok(output)
    }

    pub fn read_chunk(&mut self) -> anyhow::Result<Vec<u8>> {
        let mut buffer = [0_u8; 8192];
        let reader = self
            .reader
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("terminal reader already detached"))?;
        let count = reader.read(&mut buffer)?;
        Ok(buffer[..count].to_vec())
    }

    pub fn resize(&self, cols: u16, rows: u16) -> anyhow::Result<()> {
        if cols == 0 || rows == 0 {
            anyhow::bail!("terminal dimensions must be non-zero");
        }
        self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }

    #[cfg(unix)]
    pub fn resume(&self) -> anyhow::Result<()> {
        let pid = self
            .child
            .process_id()
            .ok_or_else(|| anyhow::anyhow!("terminal process has no pid"))?;
        let result = unsafe { libc::kill(pid as libc::pid_t, libc::SIGCONT) };
        if result != 0 {
            anyhow::bail!(
                "cannot resume terminal process: {}",
                std::io::Error::last_os_error()
            );
        }
        Ok(())
    }

    #[cfg(not(unix))]
    pub fn resume(&self) -> anyhow::Result<()> {
        anyhow::bail!("terminal resume is not available on this platform")
    }

    pub fn kill(&mut self) -> anyhow::Result<()> {
        self.child.kill()?;
        Ok(())
    }

    pub fn write(&mut self, data: &[u8]) -> anyhow::Result<()> {
        self.writer.write_all(data)?;
        self.writer.flush()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Session {
    pub name: String,
    pub session_type: String,
    pub host: String,
    pub username: String,
    pub port: u16,
    /// Secret-free reference to an OS keychain entry containing a password or key passphrase.
    #[serde(default)]
    pub credential_ref: String,
    /// Stable user-facing identity name. Secrets remain in the OS credential store.
    #[serde(default)]
    pub identity_name: String,
    /// Local path to a private key. The key material itself is never persisted here.
    #[serde(default)]
    pub private_key: String,
    #[serde(default)]
    pub environment: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub startup_command: String,
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub folder: String,
    #[serde(default)]
    pub working_directory: String,
    #[serde(default)]
    pub shell: String,
    #[serde(default)]
    pub proxy_jump: String,
    #[serde(default)]
    pub host_key_policy: String,
    #[serde(default = "default_keepalive_seconds")]
    pub keepalive_seconds: u16,
    #[serde(default)]
    pub serial_baud: u32,
    #[serde(default)]
    pub serial_data_bits: u8,
    #[serde(default)]
    pub serial_parity: String,
    #[serde(default)]
    pub serial_stop_bits: u8,
    #[serde(default)]
    pub serial_flow_control: String,
    #[serde(default)]
    pub wsl_user: String,
    #[serde(default)]
    pub wsl_command: String,
    #[serde(default)]
    pub telnet_mode: String,
    #[serde(default)]
    pub unix_shell: String,
    #[serde(default)]
    pub unix_runtime_root: String,
    #[serde(default)]
    pub unix_host_mount: String,
    #[serde(default)]
    pub rdp_width: u16,
    #[serde(default)]
    pub rdp_height: u16,
    #[serde(default)]
    pub rdp_fullscreen: bool,
    #[serde(default)]
    pub rdp_audio: bool,
    #[serde(default)]
    pub vnc_display: u16,
    #[serde(default)]
    pub vnc_view_only: bool,
    #[serde(default)]
    pub vnc_shared: bool,
    #[serde(default)]
    pub ssh_agent_monitor: bool,
    #[serde(default)]
    pub ssh_agent_forwarding: bool,
    #[serde(default)]
    pub remote_shell: String,
    #[serde(default)]
    pub favorite: bool,
}

fn default_keepalive_seconds() -> u16 {
    30
}

/// Rejects a host or user name that a command-line client could mistake for an option, or that is not a single token.
pub fn check_target(what: &'static str, value: &str) -> Result<(), ValidationError> {
    if value.starts_with('-') || value.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(ValidationError::UnsafeTarget(what));
    }
    Ok(())
}

impl Session {
    pub fn ssh(
        name: impl Into<String>,
        host: impl Into<String>,
        username: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            session_type: "SSH".into(),
            host: host.into(),
            username: username.into(),
            port: 22,
            environment: std::collections::HashMap::new(),
            startup_command: String::new(),
            group: String::new(),
            host_key_policy: "ask".into(),
            keepalive_seconds: 30,
            favorite: false,
            ..Default::default()
        }
    }

    pub fn ssh_plan(&self) -> Result<CommandPlan, ValidationError> {
        if self.host.trim().is_empty() {
            return Err(ValidationError::EmptyHost);
        }
        if self.username.trim().is_empty() {
            return Err(ValidationError::EmptyUsername);
        }
        if self.port == 0 {
            return Err(ValidationError::InvalidPort);
        }
        // A value that starts with '-' is read by ssh as an option (for example -oProxyCommand=...), which runs a local command.
        check_target("host", self.host.trim())?;
        check_target("username", self.username.trim())?;
        let mut argv = vec!["ssh".into(), "-p".into(), self.port.to_string().into()];
        if !self.proxy_jump.trim().is_empty() {
            argv.extend(["-J".into(), self.proxy_jump.trim().to_string()]);
        }
        let policy = match self.host_key_policy.trim().to_ascii_lowercase().as_str() {
            "strict" => "yes",
            "accept-new" => "accept-new",
            _ => "ask",
        };
        argv.extend(["-o".into(), format!("StrictHostKeyChecking={policy}")]);
        if self.keepalive_seconds > 0 {
            argv.extend([
                "-o".into(),
                format!("ServerAliveInterval={}", self.keepalive_seconds),
                "-o".into(),
                "ServerAliveCountMax=3".into(),
            ]);
        }
        if self.ssh_agent_forwarding {
            argv.extend(["-o".into(), "ForwardAgent=yes".into()]);
        }
        if !self.private_key.trim().is_empty() {
            check_target("private_key", self.private_key.trim())?;
            argv.extend(["-i".into(), self.private_key.trim().to_string().into()]);
        }
        // `--` ends option parsing, so the destination can never be taken for an option even if the checks above change.
        argv.push("--".into());
        argv.push(format!("{}@{}", self.username.trim(), self.host.trim()));
        match self.remote_shell.trim().to_ascii_lowercase().as_str() {
            "fish" => argv.extend(["fish".into(), "-l".into()]),
            "fish-or-sh" | "fish-fallback" => argv.extend([
                "sh".into(),
                "-lc".into(),
                "if command -v fish >/dev/null 2>&1; then exec fish -l; else exec sh -l; fi".into(),
            ]),
            "bash" => argv.extend(["bash".into(), "-l".into()]),
            "zsh" => argv.extend(["zsh".into(), "-l".into()]),
            "sh" => argv.extend(["sh".into(), "-l".into()]),
            "" | "default" | "server" => {}
            _ => return Err(ValidationError::InvalidRemoteShell),
        }
        Ok(CommandPlan::new(argv))
    }

    pub fn external_plan(&self, program: &str) -> Result<CommandPlan, ValidationError> {
        if self.host.trim().is_empty() {
            return Err(ValidationError::EmptyHost);
        }
        check_target("host", self.host.trim())?;
        if !self.username.trim().is_empty() {
            check_target("username", self.username.trim())?;
        }
        let mut argv = vec![program.to_string()];
        if !self.username.trim().is_empty() {
            argv.extend(["-l".into(), self.username.trim().to_string()]);
        }
        argv.push("--".into());
        argv.push(self.host.trim().to_string());
        if self.port != 0 {
            argv.push(self.port.to_string());
        }
        Ok(CommandPlan::new(argv))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPlan {
    pub argv: Vec<String>,
}

impl CommandPlan {
    pub fn new(argv: Vec<String>) -> Self {
        Self { argv }
    }

    pub fn shell_command(&self) -> String {
        self.argv
            .iter()
            .map(|arg| shell_quote(arg))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn shell_quote(value: &str) -> String {
    if !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "@%_+=:,./-".contains(c))
    {
        return value.to_string();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    EmptyHost,
    EmptyUsername,
    InvalidPort,
    InvalidRemoteShell,
    /// A host or user name that the ssh client could read as an option, or that contains whitespace or control characters.
    UnsafeTarget(&'static str),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyHost => write!(f, "host cannot be empty"),
            Self::EmptyUsername => write!(f, "username cannot be empty"),
            Self::InvalidPort => write!(f, "port must be non-zero"),
            Self::UnsafeTarget(what) => write!(
                f,
                "the {what} must not start with '-' or contain spaces or control characters"
            ),
            Self::InvalidRemoteShell => write!(
                f,
                "remote shell must be fish, fish-or-sh, bash, zsh, sh, or server default"
            ),
        }
    }
}

impl std::error::Error for ValidationError {}

pub struct SessionStore {
    connection: Connection,
}

impl SessionStore {
    pub fn open(path: &str) -> rusqlite::Result<Self> {
        let connection = Connection::open(path)?;
        connection.execute(
            "CREATE TABLE IF NOT EXISTS sessions (name TEXT PRIMARY KEY, payload TEXT NOT NULL)",
            [],
        )?;
        Ok(Self { connection })
    }

    pub fn replace_all(&self, sessions: &[Session]) -> rusqlite::Result<()> {
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute("DELETE FROM sessions", [])?;
        for session in sessions {
            let payload = serde_json::to_string(session)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            transaction.execute(
                "INSERT INTO sessions(name, payload) VALUES (?1, ?2)",
                params![session.name, payload],
            )?;
        }
        transaction.commit()
    }

    pub fn list(&self) -> rusqlite::Result<Vec<Session>> {
        let mut statement = self
            .connection
            .prepare("SELECT payload FROM sessions ORDER BY name")?;
        let rows = statement.query_map([], |row| {
            let payload: String = row.get(0)?;
            serde_json::from_str(&payload).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })
        })?;
        rows.collect()
    }

    pub fn upsert(&self, session: &Session) -> rusqlite::Result<()> {
        let payload = serde_json::to_string(session)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        self.connection.execute(
            "INSERT INTO sessions(name, payload) VALUES (?1, ?2) ON CONFLICT(name) DO UPDATE SET payload=excluded.payload",
            params![session.name, payload],
        )?;
        Ok(())
    }

    pub fn delete(&self, name: &str) -> rusqlite::Result<bool> {
        Ok(self
            .connection
            .execute("DELETE FROM sessions WHERE name = ?1", params![name])?
            > 0)
    }
}

#[derive(Debug)]
pub enum ImportError {
    InvalidPort(String),
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPort(value) => write!(f, "invalid port: {value}"),
        }
    }
}

impl std::error::Error for ImportError {}

pub fn import_ini(text: &str) -> Result<Vec<Session>, ImportError> {
    let mut result = Vec::new();
    let mut section: Option<String> = None;
    let mut fields = std::collections::HashMap::new();
    let flush = |section: &Option<String>,
                 fields: &mut std::collections::HashMap<String, String>,
                 result: &mut Vec<Session>|
     -> Result<(), ImportError> {
        let Some(raw_name) = section else {
            return Ok(());
        };
        if !raw_name.to_ascii_lowercase().starts_with("session") {
            return Ok(());
        }
        let kind = fields.get("type").map(String::as_str).unwrap_or("SSH");
        let session_type = match kind.to_ascii_lowercase().as_str() {
            "ssh" | "sftp" | "scp" => "SSH",
            "rdp" => "Rdp",
            "wsl" => "WSL",
            _ => return Ok(()),
        };
        let name = raw_name
            .strip_prefix("Session ")
            .unwrap_or(raw_name)
            .trim()
            .to_string();
        let host = fields.get("host").cloned().unwrap_or_default();
        let username = fields
            .get("user")
            .or_else(|| fields.get("username"))
            .cloned()
            .unwrap_or_default();
        let port = match fields.get("port") {
            Some(value) if !value.is_empty() => value
                .parse()
                .map_err(|_| ImportError::InvalidPort(value.clone()))?,
            _ => {
                if session_type == "Rdp" {
                    3389
                } else {
                    22
                }
            }
        };
        if !host.is_empty() {
            result.push(Session {
                name,
                session_type: session_type.into(),
                host,
                username,
                port,
                environment: std::collections::HashMap::new(),
                startup_command: String::new(),
                group: String::new(),
                favorite: false,
                ..Default::default()
            });
        }
        Ok(())
    };
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            flush(&section, &mut fields, &mut result)?;
            section = Some(line[1..line.len() - 1].to_string());
            fields.clear();
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_ascii_lowercase();
            if ![
                "password",
                "passphrase",
                "privatekey",
                "private_key",
                "token",
                "credential",
            ]
            .contains(&key.as_str())
            {
                fields.insert(key, value.trim().to_string());
            }
        }
    }
    flush(&section, &mut fields, &mut result)?;
    Ok(result)
}

pub fn import_bookmarks(text: &str) -> Result<Vec<Session>, ImportError> {
    let mut result = Vec::new();
    let mut in_bookmarks = false;
    let mut group = String::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            let section = &line[1..line.len() - 1];
            in_bookmarks =
                section.eq_ignore_ascii_case("Bookmarks") || section.starts_with("Bookmarks_");
            group.clear();
            continue;
        }
        if in_bookmarks && line.to_ascii_lowercase().starts_with("subrep=") {
            group = line
                .split_once('=')
                .map(|(_, value)| value.trim().replace('\\', "/"))
                .unwrap_or_default();
            continue;
        }
        if !in_bookmarks {
            continue;
        }
        let Some((name, encoded)) = line.split_once('=') else {
            continue;
        };
        if !encoded.starts_with('#') {
            continue;
        }
        let Some(first_hash) = encoded[1..].find('#') else {
            continue;
        };
        let mode_start = first_hash + 2;
        let Some(mode_end_rel) = encoded[mode_start..].find('%') else {
            continue;
        };
        let mode = &encoded[mode_start..mode_start + mode_end_rel];
        let fields: Vec<&str> = encoded[mode_start + mode_end_rel + 1..]
            .split('%')
            .collect();
        let session_type = match mode {
            "0" | "7" => "SSH",
            "4" => "Rdp",
            "5" => "Vnc",
            "14" => "WSL",
            _ => continue,
        };
        if session_type == "WSL" {
            let distribution = fields.first().copied().unwrap_or_default();
            result.push(Session {
                name: name.to_string(),
                session_type: "WSL".into(),
                host: distribution.to_string(),
                username: String::new(),
                port: 0,
                environment: std::collections::HashMap::new(),
                startup_command: String::new(),
                group: group.clone(),
                folder: group.clone(),
                favorite: false,
                ..Default::default()
            });
            continue;
        }
        if fields.len() < 3 || fields[0].is_empty() || fields[1].is_empty() {
            continue;
        }
        let port = match fields[1].parse() {
            Ok(port) => port,
            Err(_) => continue,
        };
        result.push(Session {
            name: name.to_string(),
            session_type: session_type.into(),
            host: fields[0].to_string(),
            username: fields[2].to_string(),
            port,
            environment: std::collections::HashMap::new(),
            startup_command: String::new(),
            group: group.clone(),
            folder: group.clone(),
            favorite: false,
            ..Default::default()
        });
    }
    Ok(result)
}
