#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod detach;
mod history;
mod state;
mod debug;
mod credentials;
mod ssh_agent;
mod sessions;
mod tunnel;
mod system;
mod known_hosts;
mod sftp;
mod nettools;
mod recording;
mod embedded;
mod terminal;
#[cfg(test)]
mod tests;

use tauri::{Emitter, Manager, WindowEvent};
#[cfg(test)]
#[allow(unused_imports)]
use {embedded::*, known_hosts::*, nettools::*, sftp::*, ssh_agent::*, terminal::*};
use crate::state::{TerminalManager, TunnelManager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(TerminalManager::default())
        .manage(TunnelManager::default())
        .on_window_event(|window, event| {
            if let WindowEvent::Destroyed = event {
                let label = window.label();
                if let Some(id) = detach::terminal_id_of_label(label) {
                    // The separate window is gone: tell the main window so its tab shows the terminal again.
                    let _ = window
                        .app_handle()
                        .emit("terminal-attached", id.to_string());
                } else if label == "main" {
                    // Separate terminal windows must not keep the application alive once the main window is closed.
                    window.app_handle().exit(0);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            terminal::detach_terminal,
            terminal::attach_terminal,
            terminal::terminal_replay,
            sessions::list_sessions,
            sessions::import_sessions,
            sessions::export_sessions,
            sessions::import_backup,
            sessions::save_session,
            sessions::delete_session,
            debug::configure_debug_logging,
            debug::debug_log,
            debug::debug_log_path,
            credentials::credential_status,
            ssh_agent::ssh_agent_status,
            credentials::credential_store,
            credentials::credential_delete,
            credentials::vault_encrypt,
            credentials::vault_decrypt,
            credentials::vault_bundle_export,
            credentials::vault_bundle_import,
            credentials::ssh_public_key_plan,
            ssh_agent::ssh_fido2_status,
            credentials::identity_status,
            credentials::identity_store,
            credentials::identity_delete,
            credentials::sync_credential_status,
            credentials::sync_credential_store,
            credentials::sync_credential_delete,
            credentials::sync_credential_get,
            tunnel::start_tunnel,
            tunnel::stop_tunnel,
            system::xserver_status,
            system::local_list,
            system::local_preview,
            system::local_write_text,
            system::open_local_path,
            system::shell_available,
            system::protocol_client_available,
            system::wsl_distributions,
            system::wsl_users,
            system::serial_devices,
            sftp::sftp_preview,
            known_hosts::ssh_known_hosts_status,
            known_hosts::ssh_known_hosts_remove,
            known_hosts::ssh_known_hosts_scan,
            known_hosts::ssh_known_hosts_add,
            sftp::sftp_list,
            sftp::sftp_transfer,
            sftp::start_sftp_transfer,
            sftp::cancel_sftp_transfer,
            sftp::sftp_operation,
            nettools::run_network_tool,
            nettools::scan_ports,
            recording::start_recording,
            recording::stop_recording,
            terminal::start_terminal,
            terminal::write_terminal,
            terminal::resize_terminal,
            terminal::resume_terminal,
            terminal::close_terminal,
            terminal::run_terminal_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}

fn main() {
    run();
}
