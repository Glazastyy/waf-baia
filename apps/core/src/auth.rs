use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use rand::distr::{Alphanumeric, SampleString};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapState {
    NoAdmin,
    AdminExists,
}

#[derive(Debug, Clone)]
pub struct BootstrapAdmin {
    pub username: String,
    pub password_hash: String,
    pub temporary_password: TemporaryPassword,
    pub password_change_required: bool,
    pub email_required: bool,
}

#[derive(Debug, Clone)]
pub struct TemporaryPassword {
    value: Arc<Mutex<Option<String>>>,
}

impl PartialEq for BootstrapAdmin {
    fn eq(&self, other: &Self) -> bool {
        self.username == other.username
            && self.password_change_required == other.password_change_required
            && self.email_required == other.email_required
    }
}

impl BootstrapAdmin {
    pub fn new(username: &str) -> Self {
        let mut rng = rand::rng();
        let password = Alphanumeric.sample_string(&mut rng, 32);
        let salt_material = Alphanumeric.sample_string(&mut rng, 32);
        let salt =
            SaltString::from_b64(&salt_material).expect("generated salt must be valid PHC base64");
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .expect("argon2id password hashing must be available")
            .to_string();

        Self {
            username: username.to_string(),
            password_hash,
            temporary_password: TemporaryPassword {
                value: Arc::new(Mutex::new(Some(password))),
            },
            password_change_required: true,
            email_required: true,
        }
    }
}

impl TemporaryPassword {
    pub fn expose_for_initial_log_once(&self) -> String {
        let mut value = self
            .value
            .lock()
            .expect("temporary password lock must not be poisoned");
        value.take().unwrap_or_default()
    }
}

pub fn bootstrap_initial_admin(state: BootstrapState) -> Option<BootstrapAdmin> {
    match state {
        BootstrapState::NoAdmin => Some(BootstrapAdmin::new("admin")),
        BootstrapState::AdminExists => None,
    }
}
