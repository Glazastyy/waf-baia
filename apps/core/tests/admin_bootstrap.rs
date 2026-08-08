use baia_core::auth::{BootstrapAdmin, BootstrapState, bootstrap_initial_admin};

#[test]
fn bootstrap_generates_single_use_temporary_admin_when_missing() {
    let state = BootstrapState::NoAdmin;

    let admin = bootstrap_initial_admin(state).expect("admin should be generated");

    assert_eq!(admin.username, "admin");
    assert!(admin.password_change_required);
    assert!(admin.email_required);
    assert!(admin.temporary_password.expose_for_initial_log_once().len() >= 24);
}

#[test]
fn bootstrap_is_noop_when_admin_exists() {
    let state = BootstrapState::AdminExists;

    let admin = bootstrap_initial_admin(state);

    assert_eq!(admin, None);
}

#[test]
fn temporary_password_can_only_be_exposed_once() {
    let admin = BootstrapAdmin::new("admin");

    let first = admin.temporary_password.expose_for_initial_log_once();
    let second = admin.temporary_password.expose_for_initial_log_once();

    assert!(!first.is_empty());
    assert!(second.is_empty());
}
