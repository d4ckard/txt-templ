CREATE TABLE templates (
    user_id uuid NOT NULL REFERENCES users(user_id),
    templates TEXT[] NOT NULL,
    PRIMARY KEY(user_id)
)