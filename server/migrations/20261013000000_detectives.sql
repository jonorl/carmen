-- One row per detective. There are no accounts: the name is the identity,
-- matched without regard to case (name_key), as the game shows it (name).
CREATE TABLE detectives (
    name_key   text PRIMARY KEY,
    name       text NOT NULL,
    arrests    integer NOT NULL DEFAULT 0 CHECK (arrests >= 0),
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);
