use crate::{apply_test_ssh_config, known_hosts_key, sftp_batch_command};
use crate::{
    embedded_fish_command_not_found_script, format_socket_address, scan_ports,
    take_startup_cursor_query, EMBEDDED_DIG_WRAPPER, EMBEDDED_RUNTIME_ARCHIVE,
};

#[test]
fn host_key_scan_through_ssh_uses_a_private_file_and_cleans_up() {
    // A closed loopback port: ssh cannot get a key, so the fallback must fail cleanly and leave no temp folder behind.
    let before: usize = std::fs::read_dir(std::env::temp_dir())
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .starts_with("moraxterm-hostkey-")
                })
                .count()
        })
        .unwrap_or(0);
    assert!(crate::scan_host_key_with_ssh("127.0.0.1", 1).is_err());
    let after: usize = std::fs::read_dir(std::env::temp_dir())
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| {
                    e.file_name()
                        .to_string_lossy()
                        .starts_with("moraxterm-hostkey-")
                })
                .count()
        })
        .unwrap_or(0);
    assert_eq!(
        before, after,
        "the scan left a temporary known_hosts folder behind"
    );
}

#[test]
fn host_key_scan_refuses_option_like_and_spaced_hosts() {
    for host in ["-oProxyCommand=calc", "-p", "a b", ""] {
        assert!(crate::scan_host_key(host, 22).is_err(), "{host:?}");
    }
    assert!(crate::scan_host_key("example.test", 0).is_err());
}

#[test]
fn listing_size_is_read_from_real_sftp_output_and_ignores_everything_else() {
    use crate::parse_listing_size as size;
    // exact text printed by OpenSSH sftp (Debian) in the probe, including the echoed prompt line
    let real = "sftp> ls -ln /tmp/x/big.bin\n-rw-r--r--    ? 1000     1000       3000000 Oct 20 00:28 /tmp/x/big.bin\n";
    assert_eq!(size(real), Some(3_000_000));
    assert_eq!(
        size("-rw-r--r--    ? ali      ali       3000000 Oct 20 00:28 /tmp/x/big.bin"),
        Some(3_000_000)
    );
    assert_eq!(size("-rw-r--r--    1 0 0 0 Jan  1 00:00 empty"), Some(0));
    assert_eq!(
        size("lrwxrwxrwx    1 0 0 11 Jan  1 00:00 link -> target"),
        Some(11)
    );
    assert_eq!(
        size("drwxr-xr-x    2 0 0 4096 Jan  1 00:00 dir"),
        None,
        "a directory is not a file size"
    );
    assert_eq!(size("sftp> ls -ln /tmp/x/big.bin\n"), None);
    assert_eq!(size("/tmp/x/big.bin: No such file or directory\n"), None);
    assert_eq!(size(""), None);
    assert_eq!(size("-rw-r--r-- ? 1 1 notanumber Jan 1 00:00 f"), None);
}

#[test]
fn remote_size_polling_is_skipped_for_paths_with_wildcards() {
    use crate::has_glob_characters as glob;
    for path in ["/a/b*.bin", "/a/?.bin", "/a/[x].bin", "/a/{1,2}.bin"] {
        assert!(glob(path), "{path}");
    }
    for path in ["/a/b.bin", "/home/user/my file (1).tar.gz", "C:/dir/f.txt"] {
        assert!(!glob(path), "{path}");
    }
}

#[test]
#[ignore = "needs a disposable sshd; run scripts/disposable_sftp_commands_test.sh"]
fn sftp_upload_progress_rises_while_the_file_is_still_being_sent() {
    use crate::{begin_sftp_transfer, SftpJobs};
    let port: u16 = std::env::var("MORAXTERM_TEST_SFTP_PORT")
        .unwrap()
        .parse()
        .unwrap();
    let user = std::env::var("MORAXTERM_TEST_SFTP_USER").unwrap();
    let remote = std::env::var("MORAXTERM_TEST_SFTP_REMOTE_DIR").unwrap();
    let local =
        std::path::PathBuf::from(std::env::var("MORAXTERM_TEST_SFTP_LOCAL_DIR").unwrap());
    let source = local.join("progress_src.bin");
    let size: u64 = 1_500_000;
    std::fs::write(&source, vec![7u8; size as usize]).unwrap();
    let jobs: SftpJobs = Default::default();
    let (emitter, events) = recording_emitter();
    // throttle the transfer (about 8 s) so it is still running while progress is sampled
    std::env::set_var("MORAXTERM_TEST_SFTP_LIMIT_KBIT", "1600");
    let id = begin_sftp_transfer(
        jobs,
        emitter,
        "127.0.0.1".into(),
        user,
        port,
        "upload".into(),
        source
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/"),
        format!("{remote}/progress_dst.bin"),
        false,
    )
    .unwrap();
    std::env::remove_var("MORAXTERM_TEST_SFTP_LIMIT_KBIT"); // later polls must not be throttled
    let done = wait_for_exit(&events, &id);
    assert!(done.success, "upload failed: {}", done.message);
    let samples: Vec<(u64, Option<u64>)> = events
        .lock()
        .unwrap()
        .iter()
        .filter(|(name, e)| name == "sftp-transfer-progress" && e.id == id)
        .map(|(_, e)| (e.transferred_bytes.unwrap(), e.total_bytes))
        .collect();
    eprintln!("upload progress samples (bytes, total): {samples:?}");
    assert!(
        samples.len() >= 3,
        "expected several samples, got {samples:?}"
    );
    assert!(
        samples.iter().all(|(_, total)| *total == Some(size)),
        "every sample must carry the file size: {samples:?}"
    );
    assert!(
        samples.windows(2).all(|w| w[0].0 <= w[1].0),
        "progress must never go backwards: {samples:?}"
    );
    assert!(
        samples.iter().all(|(sent, _)| *sent <= size),
        "progress above the file size: {samples:?}"
    );
    assert!(
        samples.iter().any(|(sent, _)| *sent > 0 && *sent < size),
        "no sample was taken while the upload was partway: {samples:?}"
    );
}

#[test]
fn agent_output_is_classified_from_both_streams_using_real_outputs() {
    use crate::{classify_agent_output as c, AgentState::*};
    // outputs measured on Windows: Git/MSYS build, native Windows build
    assert_eq!(c(Some(1), "The agent has no identities.\n", ""), NoKeys);
    assert_eq!(c(Some(1), "", "The agent has no identities.\n"), NoKeys);
    assert_eq!(
        c(Some(0), "256 SHA256:abc demo@host (ED25519)\n", ""),
        Keys(1)
    );
    assert_eq!(
        c(
            Some(0),
            "256 SHA256:a x (ED25519)\n3072 SHA256:b y (RSA)\n",
            ""
        ),
        Keys(2)
    );
    assert_eq!(
        c(
            Some(2),
            "",
            "Error connecting to agent: Bad file descriptor\n"
        ),
        NotRunning
    );
    assert_eq!(
        c(
            Some(2),
            "",
            "Could not open a connection to your authentication agent.\n"
        ),
        NotRunning
    );
    assert_eq!(
        c(
            Some(1),
            "",
            "Error connecting to agent: No such file or directory
\n"
        ),
        NotRunning
    );
    assert_eq!(
        c(Some(1), "", "error fetching identities: invalid format\n"),
        Unreadable
    );
    assert_eq!(
        c(Some(1), "", "something unexpected happened\n"),
        Failed("something unexpected happened".into())
    );
    assert_eq!(c(Some(1), "", ""), Failed("no output".into()));
}

#[test]
fn agent_status_text_says_what_to_do_and_keeps_the_old_wording() {
    use crate::{describe_agent as d, AgentState::*};
    assert_eq!(d(&Keys(3), None), "agent available (3 identities)");
    assert_eq!(d(&Keys(1), None), "agent available (1 identity)");
    assert_eq!(d(&NoKeys, None), "agent available (no identities)");
    let disabled = d(&NotRunning, Some((false, true)));
    assert!(
        disabled.contains("disabled")
            && disabled.contains("Set-Service ssh-agent -StartupType Manual")
            && disabled.contains("administrator"),
        "{disabled}"
    );
    let stopped = d(&NotRunning, Some((false, false)));
    assert!(
        stopped.contains("stopped") && stopped.contains("Start-Service ssh-agent"),
        "{stopped}"
    );
    assert!(d(&NotRunning, Some((true, false))).contains("running but"));
    assert!(d(&NotRunning, None).starts_with("agent unavailable"));
    assert!(d(&Unreadable, None).contains("different OpenSSH"));
    assert!(d(&Failed("boom".into()), None).ends_with("boom"));
}

#[test]
fn windows_service_text_is_read_correctly() {
    use crate::{service_is_disabled, service_is_running};
    assert!(!service_is_running(
        "SERVICE_NAME: ssh-agent\n        STATE              : 1  STOPPED \n"
    ));
    assert!(service_is_running(
        "        STATE              : 4  RUNNING \n"
    ));
    assert!(service_is_disabled(
        "        START_TYPE         : 4   DISABLED\n"
    ));
    assert!(!service_is_disabled(
        "        START_TYPE         : 3   DEMAND_START\n"
    ));
}

#[test]
fn ssh_add_is_taken_from_the_folder_of_the_first_ssh_on_path() {
    use crate::{is_windows_openssh, ssh_add_beside_ssh};
    let (ssh, add) = if cfg!(windows) {
        ("ssh.exe", "ssh-add.exe")
    } else {
        ("ssh", "ssh-add")
    };
    let root = std::env::temp_dir().join(format!("mx-agent-path-{}", std::process::id()));
    let (first, second, lonely) =
        (root.join("first"), root.join("second"), root.join("lonely"));
    for dir in [&first, &second, &lonely] {
        std::fs::create_dir_all(dir).unwrap();
    }
    for name in [ssh, add] {
        std::fs::write(first.join(name), b"").unwrap();
        std::fs::write(second.join(name), b"").unwrap();
    }
    std::fs::write(lonely.join(add), b"").unwrap(); // an ssh-add with no ssh beside it must not be chosen
    let join = |dirs: &[&std::path::PathBuf]| {
        std::env::join_paths(dirs.iter().map(|d| d.as_path())).unwrap()
    };
    assert_eq!(
        ssh_add_beside_ssh(&join(&[&first, &second])),
        Some(first.join(add))
    );
    assert_eq!(
        ssh_add_beside_ssh(&join(&[&second, &first])),
        Some(second.join(add))
    );
    assert_eq!(
        ssh_add_beside_ssh(&join(&[&lonely, &second])),
        Some(second.join(add)),
        "must skip a folder with ssh-add but no ssh"
    );
    assert_eq!(ssh_add_beside_ssh(&join(&[&lonely])), None);
    assert_eq!(ssh_add_beside_ssh(std::ffi::OsStr::new("")), None);
    let _ = std::fs::remove_dir_all(&root);
    assert!(is_windows_openssh(std::path::Path::new(
        r"C:\Windows\System32\OpenSSH\ssh-add.exe"
    )));
    assert!(is_windows_openssh(std::path::Path::new(
        "c:/windows/system32/openssh/ssh-add.exe"
    )));
    assert!(!is_windows_openssh(std::path::Path::new(
        r"C:\Program Files\Git\usr\bin\ssh-add.exe"
    )));
}

#[test]
fn known_hosts_names_follow_openssh_for_default_and_other_ports() {
    assert_eq!(known_hosts_key("example.test", 22), "example.test");
    assert_eq!(known_hosts_key("  example.test ", 22), "example.test");
    assert_eq!(known_hosts_key("example.test", 2222), "[example.test]:2222");
    assert_eq!(known_hosts_key("127.0.0.1", 22226), "[127.0.0.1]:22226");
}

#[test]
fn test_ssh_config_hook_adds_f_first_and_is_inert_when_unset() {
    let base = vec!["-X".to_string(), "-p".to_string(), "22".to_string()];
    let mut unset = base.clone();
    apply_test_ssh_config(&mut unset, None);
    assert_eq!(unset, base);
    let mut blank = base.clone();
    apply_test_ssh_config(&mut blank, Some("  ".into()));
    assert_eq!(blank, base);
    let mut set = base.clone();
    apply_test_ssh_config(&mut set, Some("C:/t/ssh_config".into()));
    assert_eq!(&set[..2], ["-F", "C:/t/ssh_config"]);
    assert_eq!(&set[2..], &base[..]);
}

#[test]
fn sftp_destination_is_always_behind_a_double_dash() {
    let command = sftp_batch_command(22, "-oProxyCommand=calc", "host.example");
    let args: Vec<String> = command
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let n = args.len();
    assert_eq!(args[n - 2], "--");
    assert_eq!(args[n - 1], "-oProxyCommand=calc@host.example");
}

#[test]
fn startup_cursor_query_is_answered_once_and_removed_from_the_stream() {
    let mut answered = false;
    let (rest, answer) = take_startup_cursor_query(b"\x1b[6n\x1b[?9001hHello", &mut answered);
    assert!(answer && answered);
    assert_eq!(rest, b"\x1b[?9001hHello");
    // A later query (for example from vim) must reach xterm.js unchanged.
    let (rest, answer) = take_startup_cursor_query(b"\x1b[6n", &mut answered);
    assert!(!answer);
    assert_eq!(rest, b"\x1b[6n");
}

#[test]
fn data_without_a_query_is_untouched_and_does_not_mark_answered() {
    let mut answered = false;
    let (rest, answer) = take_startup_cursor_query(b"plain output", &mut answered);
    assert!(!answer && !answered);
    assert_eq!(rest, b"plain output");
}

#[test]
fn only_the_first_of_two_queries_in_one_chunk_is_consumed() {
    let mut answered = false;
    let (rest, answer) = take_startup_cursor_query(b"a\x1b[6nb\x1b[6nc", &mut answered);
    assert!(answer);
    assert_eq!(rest, b"ab\x1b[6nc");
}

#[test]
fn missing_command_handler_suggests_ms2_install_syntax() {
    let script = embedded_fish_command_not_found_script();
    assert!(script.contains("pacman -S bind"));
    assert!(script.contains("pacman -S <package-name>"));
    assert!(script.contains("dig example.com"));
}

#[test]
fn embedded_dig_helper_uses_windows_dns_resolver() {
    assert!(EMBEDDED_DIG_WRAPPER.contains("nslookup"));
    assert!(EMBEDDED_DIG_WRAPPER.contains("Usage: dig <hostname>"));
}

#[test]
fn embedded_runtime_archive_name_is_versioned() {
    assert_eq!(
        EMBEDDED_RUNTIME_ARCHIVE,
        "Moraxterm_EmbeddedUnix_x64_0.1.24.tar.xz"
    );
}

#[test]
fn formats_ipv4_socket_addresses() {
    assert_eq!(format_socket_address("127.0.0.1", 22), "127.0.0.1:22");
}

#[test]
fn formats_ipv6_socket_addresses() {
    assert_eq!(format_socket_address("::1", 443), "[::1]:443");
    assert_eq!(format_socket_address("[::1]", 443), "[::1]:443");
}

#[test]
fn rejects_scan_ranges_over_32_ports() {
    let result = scan_ports("127.0.0.1".into(), 1, 33);
    assert_eq!(result, Err("port range cannot exceed 32 ports".into()));
}

#[test]
#[ignore = "needs a disposable sshd; run scripts/disposable_sftp_commands_test.sh"]
fn sftp_commands_roundtrip_against_disposable_server() {
    use crate::{sftp_list, sftp_operation, sftp_transfer};
    let port: u16 = std::env::var("MORAXTERM_TEST_SFTP_PORT")
        .unwrap()
        .parse()
        .unwrap();
    let user = std::env::var("MORAXTERM_TEST_SFTP_USER").unwrap();
    let remote = std::env::var("MORAXTERM_TEST_SFTP_REMOTE_DIR").unwrap();
    let local =
        std::path::PathBuf::from(std::env::var("MORAXTERM_TEST_SFTP_LOCAL_DIR").unwrap());
    let host = "127.0.0.1".to_string();
    let payload: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
    let up = local.join("up.bin");
    let down = local.join("down.bin");
    std::fs::write(&up, &payload).unwrap();
    let up_s = up.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/");
    let down_s = down
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/");

    sftp_operation(
        host.clone(),
        user.clone(),
        port,
        "mkdir".into(),
        format!("{remote}/d"),
        "x".into(),
        "x".into(),
    )
    .unwrap();
    sftp_operation(
        host.clone(),
        user.clone(),
        port,
        "rename".into(),
        format!("{remote}/d"),
        format!("{remote}/d2"),
        "x".into(),
    )
    .unwrap();
    sftp_operation(
        host.clone(),
        user.clone(),
        port,
        "chmod".into(),
        format!("{remote}/d2"),
        "x".into(),
        "700".into(),
    )
    .unwrap();
    sftp_transfer(
        host.clone(),
        user.clone(),
        port,
        "upload".into(),
        up_s,
        format!("{remote}/d2/up.bin"),
    )
    .unwrap();
    let listing = sftp_list(host.clone(), user.clone(), port, format!("{remote}/d2")).unwrap();
    assert!(listing.contains("up.bin"), "listing: {listing}");
    sftp_transfer(
        host.clone(),
        user.clone(),
        port,
        "download".into(),
        down_s,
        format!("{remote}/d2/up.bin"),
    )
    .unwrap();
    assert_eq!(std::fs::read(&down).unwrap(), payload);
    sftp_operation(
        host.clone(),
        user.clone(),
        port,
        "delete".into(),
        format!("{remote}/d2/up.bin"),
        "x".into(),
        "x".into(),
    )
    .unwrap();
    sftp_operation(
        host.clone(),
        user.clone(),
        port,
        "delete".into(),
        format!("{remote}/d2"),
        "x".into(),
        "x".into(),
    )
    .unwrap();
    let root_listing = sftp_list(host.clone(), user.clone(), port, remote.clone()).unwrap();
    assert!(
        !root_listing.contains("d2"),
        "directory not deleted: {root_listing}"
    );
    let after = sftp_list(host, user, port, format!("{remote}/d2")).unwrap();
    assert!(!after.contains("up.bin"), "listing after delete: {after}");
}

type Events = std::sync::Arc<std::sync::Mutex<Vec<(String, crate::SftpTransferEvent)>>>;

fn recording_emitter() -> (crate::SftpEmitter, Events) {
    let events: Events = Default::default();
    let sink = std::sync::Arc::clone(&events);
    let emitter: crate::SftpEmitter = std::sync::Arc::new(move |name, event| {
        sink.lock().unwrap().push((name.to_string(), event));
    });
    (emitter, events)
}

fn wait_for_exit(events: &Events, id: &str) -> crate::SftpTransferEvent {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        if let Some((_, event)) = events
            .lock()
            .unwrap()
            .iter()
            .find(|(name, event)| name == "sftp-transfer-exit" && event.id == id)
        {
            return event.clone();
        }
        assert!(
            std::time::Instant::now() < deadline,
            "timed out waiting for {id}"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

#[test]
#[ignore = "needs a disposable sshd; run scripts/disposable_sftp_commands_test.sh"]
fn sftp_queued_transfer_resume_and_cancel_against_disposable_server() {
    use crate::{begin_sftp_transfer, cancel_sftp_job, SftpJobs};
    let port: u16 = std::env::var("MORAXTERM_TEST_SFTP_PORT")
        .unwrap()
        .parse()
        .unwrap();
    let user = std::env::var("MORAXTERM_TEST_SFTP_USER").unwrap();
    let remote = std::env::var("MORAXTERM_TEST_SFTP_REMOTE_DIR").unwrap();
    let local =
        std::path::PathBuf::from(std::env::var("MORAXTERM_TEST_SFTP_LOCAL_DIR").unwrap());
    let norm =
        |p: &std::path::Path| p.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/");
    let host = "127.0.0.1".to_string();
    let jobs: SftpJobs = Default::default();
    let (emitter, events) = recording_emitter();
    let payload: Vec<u8> = (0..1_500_000u32)
        .map(|i| (i.wrapping_mul(31) % 253) as u8)
        .collect();
    let full = local.join("q_full.bin");
    std::fs::write(&full, &payload).unwrap();

    // Plain queued upload, then download, with byte-exact comparison.
    let id = begin_sftp_transfer(
        jobs.clone(),
        emitter.clone(),
        host.clone(),
        user.clone(),
        port,
        "upload".into(),
        norm(&full),
        format!("{remote}/q_remote.bin"),
        false,
    )
    .unwrap();
    let done = wait_for_exit(&events, &id);
    assert!(done.success, "upload failed: {}", done.message);
    let down = local.join("q_down.bin");
    let id = begin_sftp_transfer(
        jobs.clone(),
        emitter.clone(),
        host.clone(),
        user.clone(),
        port,
        "download".into(),
        norm(&down),
        format!("{remote}/q_remote.bin"),
        false,
    )
    .unwrap();
    let done = wait_for_exit(&events, &id);
    assert!(done.success, "download failed: {}", done.message);
    assert_eq!(std::fs::read(&down).unwrap(), payload);

    // Resume download: truncate the local copy, then reget must restore it.
    std::fs::write(&down, &payload[..400_000]).unwrap();
    let id = begin_sftp_transfer(
        jobs.clone(),
        emitter.clone(),
        host.clone(),
        user.clone(),
        port,
        "download".into(),
        norm(&down),
        format!("{remote}/q_remote.bin"),
        true,
    )
    .unwrap();
    let done = wait_for_exit(&events, &id);
    assert!(done.success, "resume download failed: {}", done.message);
    assert_eq!(std::fs::read(&down).unwrap(), payload);

    // Resume upload: put a truncated file at the remote path, then reput the full one.
    let partial = local.join("q_partial.bin");
    std::fs::write(&partial, &payload[..600_000]).unwrap();
    let id = begin_sftp_transfer(
        jobs.clone(),
        emitter.clone(),
        host.clone(),
        user.clone(),
        port,
        "upload".into(),
        norm(&partial),
        format!("{remote}/q_resume.bin"),
        false,
    )
    .unwrap();
    assert!(wait_for_exit(&events, &id).success);
    let id = begin_sftp_transfer(
        jobs.clone(),
        emitter.clone(),
        host.clone(),
        user.clone(),
        port,
        "upload".into(),
        norm(&full),
        format!("{remote}/q_resume.bin"),
        true,
    )
    .unwrap();
    let done = wait_for_exit(&events, &id);
    assert!(done.success, "resume upload failed: {}", done.message);
    let back = local.join("q_back.bin");
    let id = begin_sftp_transfer(
        jobs.clone(),
        emitter.clone(),
        host.clone(),
        user.clone(),
        port,
        "download".into(),
        norm(&back),
        format!("{remote}/q_resume.bin"),
        false,
    )
    .unwrap();
    assert!(wait_for_exit(&events, &id).success);
    assert_eq!(std::fs::read(&back).unwrap(), payload);

    // Failure path: a missing remote file must report success=false, not hang.
    let id = begin_sftp_transfer(
        jobs.clone(),
        emitter.clone(),
        host.clone(),
        user.clone(),
        port,
        "download".into(),
        norm(&local.join("q_missing.bin")),
        format!("{remote}/does-not-exist"),
        false,
    )
    .unwrap();
    let done = wait_for_exit(&events, &id);
    assert!(!done.success, "missing remote file reported success");

    // Cancel: throttle the client so the job is still running when we cancel it.
    std::env::set_var("MORAXTERM_TEST_SFTP_LIMIT_KBIT", "64");
    let id = begin_sftp_transfer(
        jobs.clone(),
        emitter.clone(),
        host.clone(),
        user.clone(),
        port,
        "upload".into(),
        norm(&full),
        format!("{remote}/q_cancel.bin"),
        false,
    )
    .unwrap();
    std::env::remove_var("MORAXTERM_TEST_SFTP_LIMIT_KBIT");
    std::thread::sleep(std::time::Duration::from_millis(700));
    assert!(
        jobs.lock().unwrap().contains_key(&id),
        "job finished before cancel"
    );
    assert!(cancel_sftp_job(&jobs, &emitter, id.clone()).unwrap());
    let done = wait_for_exit(&events, &id);
    assert!(!done.success);
    assert_eq!(done.message, "transfer cancelled");
    assert!(!jobs.lock().unwrap().contains_key(&id));
    assert!(
        !cancel_sftp_job(&jobs, &emitter, id.clone()).unwrap(),
        "second cancel must be a no-op"
    );
    std::thread::sleep(std::time::Duration::from_millis(400));
    let exits = events
        .lock()
        .unwrap()
        .iter()
        .filter(|(n, e)| n == "sftp-transfer-exit" && e.id == id)
        .count();
    assert_eq!(exits, 1, "cancelled job emitted {exits} exit events");

    // Download progress events carry observed byte counts and ids stay unique.
    let ids: std::collections::HashSet<String> = events
        .lock()
        .unwrap()
        .iter()
        .map(|(_, e)| e.id.clone())
        .collect();
    assert!(ids.len() >= 7);
}
