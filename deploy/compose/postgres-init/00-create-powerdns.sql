SELECT 'CREATE DATABASE powerdns OWNER baia'
WHERE NOT EXISTS (
    SELECT 1 FROM pg_database WHERE datname = 'powerdns'
)\gexec
