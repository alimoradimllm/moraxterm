use moraxterm_core::import_bookmarks;

#[test]
fn imports_ssh_bookmark_mode_zero() {
    let text = "[Bookmarks]\nserver-one=#109#0%server.example%60222%ali%%-1%-1%%%%%0";
    let sessions = import_bookmarks(text).unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_type, "SSH");
    assert_eq!(sessions[0].host, "server.example");
    assert_eq!(sessions[0].port, 60222);
}

#[test]
fn imports_rdp_and_wsl_modes() {
    let text = "[Bookmarks]\nrdp-box=#91#4%10.0.0.5%3389%administrator%0\nwsl-box=#149#14%2%";
    let sessions = import_bookmarks(text).unwrap();
    assert_eq!(
        sessions
            .iter()
            .map(|s| s.session_type.as_str())
            .collect::<Vec<_>>(),
        ["Rdp", "WSL"]
    );
}

#[test]
fn imports_vnc_bookmark_mode_five() {
    let text = "[Bookmarks]\nvnc-box=#128#5%server.example%5900%-1%0";
    let sessions = import_bookmarks(text).unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_type, "Vnc");
    assert_eq!(sessions[0].host, "server.example");
    assert_eq!(sessions[0].port, 5900);
}

#[test]
fn preserves_mobaxterm_subrep_as_normalized_group() {
    let text = "[Bookmarks_7]\nSubRep=Operations\\Linux\\Production\nnode=#109#0%server.example%22%operator%%-1";
    let sessions = import_bookmarks(text).unwrap();
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].group, "Operations/Linux/Production");
    // The interface shows folders from `folder`, so the imported path must be there too.
    assert_eq!(sessions[0].folder, "Operations/Linux/Production");
}
