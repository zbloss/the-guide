-- 005_playstyle_fk.sql: Recreate playstyle_profiles with FK + NOT NULL + DEFAULT on updated_at
-- SQLite cannot ALTER TABLE to add foreign keys, so we recreate the table.

PRAGMA foreign_keys=OFF;

CREATE TABLE playstyle_profiles_new (
    campaign_id           TEXT NOT NULL PRIMARY KEY REFERENCES campaigns(id) ON DELETE CASCADE,
    combat_affinity       REAL NOT NULL DEFAULT 0.33,
    social_affinity       REAL NOT NULL DEFAULT 0.33,
    exploration_affinity  REAL NOT NULL DEFAULT 0.34,
    preferred_difficulty  REAL NOT NULL DEFAULT 0.5,
    sessions_sampled      INTEGER NOT NULL DEFAULT 0,
    updated_at            TEXT NOT NULL DEFAULT (datetime('now'))
);

INSERT INTO playstyle_profiles_new
    SELECT
        campaign_id,
        combat_affinity,
        social_affinity,
        exploration_affinity,
        preferred_difficulty,
        sessions_sampled,
        COALESCE(updated_at, datetime('now'))
    FROM playstyle_profiles;

DROP TABLE playstyle_profiles;
ALTER TABLE playstyle_profiles_new RENAME TO playstyle_profiles;

PRAGMA foreign_keys=ON;
