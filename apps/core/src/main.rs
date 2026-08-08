use baia_core::auth::{BootstrapState, bootstrap_initial_admin};
use baia_core::config::PlatformConfig;
use baia_core::server::{ServerConfig, serve};

#[tokio::main]
async fn main() {
    let config = PlatformConfig::default();
    config
        .validate()
        .expect("platform configuration must be valid");

    if let Some(admin) = bootstrap_initial_admin(BootstrapState::AdminExists) {
        let password = admin.temporary_password.expose_for_initial_log_once();
        println!("Admin user: {}", admin.username);
        println!("Temporary password: {password}");
        println!("Login URL: {}/login", config.platform.public_url);
    }

    let server_config = ServerConfig::from_env();
    println!(
        "Baia Core control plane listening on {}",
        server_config.bind_addr
    );
    serve(server_config)
        .await
        .expect("Baia Core HTTP server must keep running");
}
