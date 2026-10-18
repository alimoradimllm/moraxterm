use moraxterm_core::Session;

#[test]
fn ssh_agent_forwarding_is_opt_in() {
    let session = Session::ssh("test", "example.test", "alice");
    let plan = session.ssh_plan().unwrap();
    assert!(!plan
        .argv
        .windows(2)
        .any(|pair| pair == ["-o", "ForwardAgent=yes"]));
}

#[test]
fn enabled_ssh_agent_forwarding_adds_forward_agent_option() {
    let mut session = Session::ssh("test", "example.test", "alice");
    session.ssh_agent_forwarding = true;
    let plan = session.ssh_plan().unwrap();
    assert!(plan
        .argv
        .windows(2)
        .any(|pair| pair == ["-o", "ForwardAgent=yes"]));
}

#[test]
fn default_remote_shell_does_not_append_a_command() {
    let session = Session::ssh("test", "example.test", "alice");
    let plan = session.ssh_plan().unwrap();
    assert_eq!(plan.argv.last().unwrap(), "alice@example.test");
}

#[test]
fn fish_remote_shell_is_structured_as_argv() {
    let mut session = Session::ssh("test", "example.test", "alice");
    session.remote_shell = "fish".into();
    let plan = session.ssh_plan().unwrap();
    assert_eq!(
        &plan.argv[plan.argv.len() - 3..],
        ["alice@example.test", "fish", "-l"]
    );
}

#[test]
fn fish_fallback_uses_remote_posix_probe() {
    let mut session = Session::ssh("test", "example.test", "alice");
    session.remote_shell = "fish-or-sh".into();
    let plan = session.ssh_plan().unwrap();
    assert_eq!(
        &plan.argv[plan.argv.len() - 3..],
        [
            "sh",
            "-lc",
            "if command -v fish >/dev/null 2>&1; then exec fish -l; else exec sh -l; fi"
        ]
    );
}

#[test]
fn proxy_jump_and_host_key_policy_are_structured_in_ssh_plan() {
    let mut session = Session::ssh("test", "target.example", "alice");
    session.proxy_jump = "jump.example".into();
    session.host_key_policy = "strict".into();
    let plan = session.ssh_plan().unwrap();
    assert!(plan
        .argv
        .windows(2)
        .any(|pair| pair == ["-J", "jump.example"]));
    assert!(plan
        .argv
        .windows(2)
        .any(|pair| pair == ["-o", "StrictHostKeyChecking=yes"]));
}

#[test]
fn destination_follows_a_double_dash_so_it_can_never_be_an_option() {
    let session = Session::ssh("test", "example.test", "alice");
    let plan = session.ssh_plan().unwrap();
    let n = plan.argv.len();
    assert_eq!(&plan.argv[n - 2..], ["--", "alice@example.test"]);
}

#[test]
fn a_username_or_host_that_looks_like_an_option_is_refused() {
    for username in ["-oProxyCommand=calc", "-J", "-"] {
        assert!(
            Session::ssh("t", "example.test", username)
                .ssh_plan()
                .is_err(),
            "username {username:?}"
        );
    }
    for host in ["-oProxyCommand=calc", "-p", "-"] {
        assert!(
            Session::ssh("t", host, "alice").ssh_plan().is_err(),
            "host {host:?}"
        );
    }
}

#[test]
fn whitespace_and_control_characters_in_the_target_are_refused() {
    for bad in ["al ice", "alice\nbob", "ali\tce", "al\u{0}ice"] {
        assert!(
            Session::ssh("t", "example.test", bad).ssh_plan().is_err(),
            "{bad:?}"
        );
    }
    assert!(Session::ssh("t", "exa mple.test", "alice")
        .ssh_plan()
        .is_err());
}

#[test]
fn surrounding_spaces_are_trimmed_not_sent() {
    let plan = Session::ssh("t", "  example.test ", " alice ")
        .ssh_plan()
        .unwrap();
    assert_eq!(plan.argv.last().unwrap(), "alice@example.test");
}

#[test]
fn ordinary_names_are_still_accepted() {
    for (host, user) in [
        ("example.test", "alice"),
        ("10.0.0.5", "root"),
        ("fe80::1", "svc_deploy"),
        ("host-1.example.test", "first.last"),
        ("example.test", "DOMAIN\\user"),
    ] {
        assert!(
            Session::ssh("t", host, user).ssh_plan().is_ok(),
            "{user}@{host}"
        );
    }
}

#[test]
fn the_fish_or_sh_form_keeps_the_destination_behind_the_double_dash() {
    let mut session = Session::ssh("test", "example.test", "alice");
    session.remote_shell = "fish-or-sh".into();
    let plan = session.ssh_plan().unwrap();
    let dd = plan.argv.iter().position(|a| a == "--").unwrap();
    assert_eq!(plan.argv[dd + 1], "alice@example.test");
    assert_eq!(plan.argv[dd + 2], "sh");
}
