CREATE TYPE priorities AS ENUM ('highest', 'high', 'medium', 'low', 'lowest');

CREATE TABLE users (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    username TEXT NOT NULL CHECK (btrim(username) <> ''),
    email TEXT NOT NULL CHECK (btrim(email) <> ''),
    password_hash TEXT NOT NULL CHECK (password_hash <> ''),
    display_name TEXT,
    about TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX users_username_unique ON users (lower(username));
CREATE UNIQUE INDEX users_email_unique ON users (lower(email));

-- Storage references identify private R2 sources, never signed delivery URLs.
-- Keep rows until object deletion succeeds so cleanup can be retried.
CREATE TABLE media_assets (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    owner_id BIGINT NOT NULL REFERENCES users(id),
    bucket TEXT NOT NULL CHECK (btrim(bucket) <> ''),
    object_key TEXT NOT NULL CHECK (btrim(object_key) <> ''),
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'ready', 'failed', 'deleting')),
    content_type TEXT CHECK (content_type IN ('image/jpeg', 'image/png', 'image/webp')),
    byte_size BIGINT CHECK (byte_size > 0),
    width INTEGER CHECK (width > 0),
    height INTEGER CHECK (height > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (bucket, object_key),
    UNIQUE (owner_id, id),
    CHECK (status <> 'ready' OR (
        content_type IS NOT NULL AND byte_size IS NOT NULL
        AND width IS NOT NULL AND height IS NOT NULL
    ))
);
CREATE INDEX media_assets_cleanup_idx ON media_assets (status, updated_at, id);

-- Composite references prevent assigning another account's uploads.
-- Missing association means the locally rendered initials fallback.
CREATE TABLE user_avatars (
    user_id BIGINT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    media_id BIGINT NOT NULL,
    focal_x NUMERIC NOT NULL DEFAULT 0.5 CHECK (focal_x BETWEEN 0 AND 1),
    focal_y NUMERIC NOT NULL DEFAULT 0.5 CHECK (focal_y BETWEEN 0 AND 1),
    FOREIGN KEY (user_id, media_id) REFERENCES media_assets(owner_id, id)
);
CREATE INDEX user_avatars_media_idx ON user_avatars (media_id);

CREATE TABLE groups (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    owner_id BIGINT NOT NULL REFERENCES users(id),
    name TEXT NOT NULL CHECK (btrim(name) <> ''),
    about TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX groups_owner_name_unique ON groups (owner_id, lower(name));

-- Group owners have implicit membership; these rows represent additional members.
CREATE TABLE user_groups (
    group_id BIGINT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (group_id, user_id)
);
CREATE INDEX user_groups_user_idx ON user_groups (user_id, group_id);

CREATE TABLE lists (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    owner_id BIGINT NOT NULL REFERENCES users(id),
    name TEXT NOT NULL CHECK (btrim(name) <> ''),
    description TEXT,
    event_date DATE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (owner_id, id)
);
CREATE INDEX lists_owner_idx ON lists (owner_id, id);

-- Slots and crop settings belong to usages, not source images. The registry
-- editor will define its allowed slots; this does not finalize its feature set.
-- Visibility follows the list's access policy, not an independent public flag.
CREATE TABLE list_media (
    list_id BIGINT NOT NULL,
    owner_id BIGINT NOT NULL,
    slot TEXT NOT NULL CHECK (btrim(slot) <> ''),
    media_id BIGINT NOT NULL,
    focal_x NUMERIC NOT NULL DEFAULT 0.5 CHECK (focal_x BETWEEN 0 AND 1),
    focal_y NUMERIC NOT NULL DEFAULT 0.5 CHECK (focal_y BETWEEN 0 AND 1),
    PRIMARY KEY (list_id, slot),
    FOREIGN KEY (owner_id, list_id) REFERENCES lists(owner_id, id) ON DELETE CASCADE,
    FOREIGN KEY (owner_id, media_id) REFERENCES media_assets(owner_id, id)
);
CREATE INDEX list_media_media_idx ON list_media (media_id);
CREATE INDEX list_media_owner_idx ON list_media (owner_id, list_id);

-- No grant rows means private. Ownership is independent of read grants.
CREATE TABLE list_user_grants (
    list_id BIGINT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    PRIMARY KEY (list_id, user_id)
);
CREATE INDEX list_user_grants_user_idx ON list_user_grants (user_id, list_id);
CREATE TABLE list_group_grants (
    list_id BIGINT NOT NULL REFERENCES lists(id) ON DELETE CASCADE,
    group_id BIGINT NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    PRIMARY KEY (list_id, group_id)
);
CREATE INDEX list_group_grants_group_idx ON list_group_grants (group_id, list_id);

CREATE TABLE items (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    list_id BIGINT NOT NULL REFERENCES lists(id),
    title TEXT NOT NULL CHECK (btrim(title) <> ''),
    retailer_url TEXT NOT NULL CHECK (retailer_url ~* '^https?://[^[:space:]]+$'),
    description TEXT,
    priority priorities NOT NULL DEFAULT 'medium',
    desired_quantity INTEGER NOT NULL DEFAULT 1 CHECK (desired_quantity > 0),
    sort_order BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    archived_at TIMESTAMPTZ
);
CREATE INDEX items_list_sort_idx ON items (list_id, sort_order, id);
CREATE INDEX items_list_priority_idx ON items (list_id, priority, id);

-- Separate notes allow either a single-note or multi-note UI without data loss.
-- Authorship follows list ownership; viewer-authored discussion is not supported.
CREATE TABLE item_notes (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    item_id BIGINT NOT NULL REFERENCES items(id) ON DELETE CASCADE,
    body TEXT NOT NULL CHECK (btrim(body) <> ''),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX item_notes_item_idx ON item_notes (item_id, id);

-- Preserve attribution/history when items are archived or quantities are reduced.
-- Corrections update quantity; cancellation sets canceled_at instead of deleting.
CREATE TABLE purchases (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    item_id BIGINT NOT NULL REFERENCES items(id),
    purchaser_id BIGINT NOT NULL REFERENCES users(id),
    quantity INTEGER NOT NULL CHECK (quantity > 0),
    submission_key TEXT NOT NULL CHECK (btrim(submission_key) <> ''),
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    canceled_at TIMESTAMPTZ,
    UNIQUE (purchaser_id, submission_key)
);
CREATE INDEX purchases_item_idx ON purchases (item_id, recorded_at);
CREATE INDEX purchases_purchaser_idx ON purchases (purchaser_id, id);
