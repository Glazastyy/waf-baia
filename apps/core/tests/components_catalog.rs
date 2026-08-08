use baia_core::components::{
    ComponentApplyMode, ComponentId, ComponentManagementSurface, ComponentSecret, component_catalog,
};

#[test]
fn component_catalog_lists_every_runtime_piece_the_core_must_manage() {
    let catalog = component_catalog();
    let ids = catalog
        .iter()
        .map(|component| component.id.clone())
        .collect::<Vec<_>>();

    assert_eq!(
        ids,
        vec![
            ComponentId::Core,
            ComponentId::Web,
            ComponentId::Caddy,
            ComponentId::Postgres,
            ComponentId::Redis,
            ComponentId::PowerDns,
            ComponentId::Cloudflare,
            ComponentId::CrowdSec,
            ComponentId::Acme,
        ]
    );
}

#[test]
fn caddy_is_managed_through_the_admin_api_and_applied_by_core() {
    let catalog = component_catalog();
    let caddy = catalog
        .iter()
        .find(|component| component.id == ComponentId::Caddy)
        .expect("Caddy must be part of the component catalog");

    assert_eq!(caddy.management_surface, ComponentManagementSurface::Core);
    assert_eq!(caddy.apply_mode, ComponentApplyMode::HotReload);
    assert!(
        caddy
            .settings
            .iter()
            .any(|setting| setting.key == "services.caddyAdminUrl")
    );
    assert!(caddy.capabilities.contains(&"reverse_proxy"));
}

#[test]
fn external_integrations_keep_secrets_out_of_public_platform_yaml() {
    let catalog = component_catalog();
    let powerdns = catalog
        .iter()
        .find(|component| component.id == ComponentId::PowerDns)
        .expect("PowerDNS must be part of the component catalog");
    let cloudflare = catalog
        .iter()
        .find(|component| component.id == ComponentId::Cloudflare)
        .expect("Cloudflare must be part of the component catalog");
    let crowdsec = catalog
        .iter()
        .find(|component| component.id == ComponentId::CrowdSec)
        .expect("CrowdSec must be part of the component catalog");

    assert!(
        powerdns
            .secrets
            .contains(&ComponentSecret::new("BAIA_POWERDNS_API_KEY"))
    );
    assert!(
        cloudflare
            .secrets
            .contains(&ComponentSecret::new("BAIA_CLOUDFLARE_API_TOKEN"))
    );
    assert!(
        crowdsec
            .secrets
            .contains(&ComponentSecret::new("BAIA_CROWDSEC_API_KEY"))
    );
    assert!(
        cloudflare
            .settings
            .iter()
            .any(|setting| setting.key == "integrations.cloudflare.automaticDns.defaultProxied")
    );
}

#[test]
fn internal_stateful_services_require_restart_and_are_not_user_facing_surfaces() {
    let catalog = component_catalog();

    for id in [ComponentId::Postgres, ComponentId::Redis] {
        let component = catalog
            .iter()
            .find(|component| component.id == id)
            .expect("stateful component must be present");

        assert_eq!(
            component.management_surface,
            ComponentManagementSurface::Core
        );
        assert_eq!(component.apply_mode, ComponentApplyMode::RestartRequired);
        assert!(component.user_facing_configuration.is_empty());
    }
}
