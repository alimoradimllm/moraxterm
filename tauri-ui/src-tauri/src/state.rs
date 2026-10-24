//! Shared application state managed by Tauri (terminals, tunnels) and the events sent to the front end.

use moraxterm_core::TerminalProcess;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::process::Child;
use std::sync::{Arc, Mutex};
use crate::history::TerminalHistory;
use crate::debug::DebugConfig;

#[derive(Default)]
pub(crate) struct TerminalManager {
    pub(crate) processes: Arc<Mutex<HashMap<String, Arc<Mutex<TerminalProcess>>>>>,
    pub(crate) recordings: Arc<Mutex<HashMap<String, File>>>,
    /// Recent output per terminal, so a window opened later (a detached terminal) can show what was already printed.
    pub(crate) history: Arc<Mutex<HashMap<String, TerminalHistory>>>,
    pub(crate) sftp_jobs: Arc<Mutex<HashMap<String, Child>>>,
    pub(crate) debug: Arc<Mutex<DebugConfig>>,
}

#[derive(Default)]
pub(crate) struct TunnelManager {
    pub(crate) processes: Mutex<HashMap<String, Child>>,
}

#[derive(Clone, Serialize)]
pub(crate) struct TerminalEvent {
    pub(crate) id: String,
    pub(crate) data: String,
    /// Bytes this terminal has printed up to and including `data` (0 on exit events). A window that loaded the history up to N
    /// bytes ignores events whose total is N or less, because that output is already in the history it loaded.
    pub(crate) total: u64,
}

#[derive(Serialize)]
pub(crate) struct TerminalReplay {
    pub(crate) data: String,
    pub(crate) total: u64,
}
