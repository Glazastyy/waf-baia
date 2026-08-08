CREATE TABLE cloudflare_dns_change_plans (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    application_id UUID REFERENCES applications(id) ON DELETE SET NULL,
    zone_id UUID REFERENCES dns_zones(id) ON DELETE SET NULL,
    hostname TEXT NOT NULL,
    cloudflare_zone_id TEXT NOT NULL,
    desired_records JSONB NOT NULL,
    warnings JSONB NOT NULL DEFAULT '[]'::jsonb,
    status TEXT NOT NULL,
    created_by UUID REFERENCES users(id) ON DELETE SET NULL,
    applied_by UUID REFERENCES users(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    applied_at TIMESTAMPTZ,
    CONSTRAINT cloudflare_dns_change_plans_status_check CHECK (status IN ('planned', 'applied', 'failed', 'cancelled'))
);

ALTER TABLE dns_records
    ADD COLUMN provider_metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN managed_by_baia BOOLEAN NOT NULL DEFAULT FALSE;
