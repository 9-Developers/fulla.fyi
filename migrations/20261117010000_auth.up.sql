CREATE TABLE auth_sessions (
    id TEXT PRIMARY KEY,
    data JSONB NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX auth_sessions_expiry_idx ON auth_sessions (expires_at);

-- Atomic, shared limits across application replicas. Keys never contain passwords.
CREATE TABLE auth_rate_limits (
    key TEXT PRIMARY KEY,
    attempts INTEGER NOT NULL CHECK (attempts > 0),
    expires_at TIMESTAMPTZ NOT NULL
);
CREATE INDEX auth_rate_limits_expiry_idx ON auth_rate_limits (expires_at);
