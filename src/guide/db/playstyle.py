from __future__ import annotations

from datetime import datetime, timezone
from uuid import UUID

import aiosqlite

from guide.models.playstyle import PlaystyleProfile


class PlaystyleProfileRepository:
    def __init__(self, db: aiosqlite.Connection) -> None:
        self._db = db

    async def get_or_default(self, campaign_id: UUID) -> PlaystyleProfile:
        async with self._db.execute(
            "SELECT * FROM playstyle_profiles WHERE campaign_id = ?", (str(campaign_id),)
        ) as cursor:
            row = await cursor.fetchone()

        if row is None:
            return PlaystyleProfile.default_for(campaign_id)

        return PlaystyleProfile(
            campaign_id=campaign_id,
            combat_affinity=row["combat_affinity"],
            social_affinity=row["social_affinity"],
            exploration_affinity=row["exploration_affinity"],
            preferred_difficulty=row["preferred_difficulty"],
            sessions_sampled=row["sessions_sampled"],
            updated_at=datetime.fromisoformat(row["updated_at"]),
        )

    async def upsert(self, profile: PlaystyleProfile) -> None:
        now = profile.updated_at.isoformat()
        await self._db.execute(
            "INSERT INTO playstyle_profiles"
            " (campaign_id, combat_affinity, social_affinity, exploration_affinity,"
            "  preferred_difficulty, sessions_sampled, updated_at)"
            " VALUES (?, ?, ?, ?, ?, ?, ?)"
            " ON CONFLICT(campaign_id) DO UPDATE SET"
            "  combat_affinity = excluded.combat_affinity,"
            "  social_affinity = excluded.social_affinity,"
            "  exploration_affinity = excluded.exploration_affinity,"
            "  preferred_difficulty = excluded.preferred_difficulty,"
            "  sessions_sampled = excluded.sessions_sampled,"
            "  updated_at = excluded.updated_at",
            (
                str(profile.campaign_id),
                profile.combat_affinity,
                profile.social_affinity,
                profile.exploration_affinity,
                profile.preferred_difficulty,
                profile.sessions_sampled,
                now,
            ),
        )
        await self._db.commit()
