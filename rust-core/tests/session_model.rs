use moraxterm_core::{CommandPlan, Session};

#[test]
fn ssh_plan_uses_default_port_without_password() {
    let session = Session::ssh("prod", "31.70.116.241", "root");
    let plan = session.ssh_plan().unwrap();
    assert_eq!(
        plan.argv,
        vec![
            "ssh",
            "-p",
            "22",
            "-o",
            "StrictHostKeyChecking=ask",
            "-o",
            "ServerAliveInterval=30",
            "-o",
            "ServerAliveCountMax=3",
            "--",
            "root@31.70.116.241",
        ]
    );
    assert!(!plan.shell_command().contains("password"));
}

#[test]
fn external_protocol_plan_is_structured_and_rejects_option_injection() {
    let session = Session {
        host: "server.example".into(),
        username: "alice".into(),
        port: 21,
        ..Default::default()
    };
    assert_eq!(
        session.external_plan("ftp").unwrap().argv,
        ["ftp", "-l", "alice", "--", "server.example", "21"]
    );
    let unsafe_session = Session {
        host: "-oProxyCommand=bad".into(),
        ..session
    };
    assert!(unsafe_session.external_plan("rsh").is_err());
}

#[test]
fn ssh_plan_selects_key_path_without_embedding_secret_material() {
    let mut session = Session::ssh("key", "example.test", "alice");
    session.private_key = "C:\\Users\\alice\\.ssh\\id_ed25519".into();
    session.credential_ref = "keyring:ssh/alice@example.test".into();
    let plan = session.ssh_plan().unwrap();
    assert!(plan
        .argv
        .windows(2)
        .any(|pair| pair == ["-i", "C:\\Users\\alice\\.ssh\\id_ed25519"]));
    assert!(!plan.shell_command().contains("keyring:ssh"));
}

#[test]
fn mosh_plan_uses_structured_ssh_bootstrap_and_safe_destination() {
    let mut session = Session::ssh("mosh", "example.test", "alice");
    session.session_type = "Mosh".into();
    session.port = 2200;
    let plan = session.mosh_plan().unwrap();
    assert_eq!(
        plan.argv,
        ["mosh", "--ssh", "ssh -p 2200", "--", "alice@example.test"]
    );
}

#[test]
fn empty_host_is_rejected() {
    let session = Session::ssh("bad", "", "root");
    assert!(session.ssh_plan().is_err());
}

#[test]
fn shell_command_quotes_arguments() {
    let plan = CommandPlan::new(vec![
        "ssh".into(),
        "root@example.test".into(),
        "printf hi; exit".into(),
    ]);
    assert_eq!(
        plan.shell_command(),
        "ssh root@example.test 'printf hi; exit'"
    );
}
