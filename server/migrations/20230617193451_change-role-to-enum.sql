CREATE TYPE character_role AS ENUM ('admin', 'player');
ALTER TABLE characters ADD COLUMN new_role character_role NOT NULL DEFAULT 'player';
ALTER TABLE characters DROP COLUMN role;
ALTER TABLE characters RENAME COLUMN new_role TO role;
