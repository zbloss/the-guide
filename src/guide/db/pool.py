from __future__ import annotations

from pathlib import Path

import aiosqlite

_db: aiosqlite.Connection | None = None


async def init_db(db_url: str) -> aiosqlite.Connection:
    """Open (or create) the SQLite database and run all migrations."""
    global _db

    # Strip sqlite:// prefix if present
    path = db_url.removeprefix("sqlite://").removeprefix("sqlite:")
    if path != ":memory:":
        Path(path).parent.mkdir(parents=True, exist_ok=True)

    db = await aiosqlite.connect(path)
    db.row_factory = aiosqlite.Row
    await db.execute("PRAGMA journal_mode=WAL")
    await db.execute("PRAGMA foreign_keys=ON")
    await _run_migrations(db)
    _db = db
    return db


async def get_db() -> aiosqlite.Connection:
    if _db is None:
        raise RuntimeError("Database not initialised — call init_db() first")
    return _db


async def close_db() -> None:
    global _db
    if _db is not None:
        await _db.close()
        _db = None


async def _run_migrations(db: aiosqlite.Connection) -> None:
    # Ensure migration tracking table exists
    await db.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (name TEXT PRIMARY KEY NOT NULL)"
    )
    await db.commit()

    migrations_dir = Path(__file__).parent / "migrations"
    for sql_file in sorted(migrations_dir.glob("*.sql")):
        name = sql_file.name
        async with db.execute(
            "SELECT 1 FROM schema_migrations WHERE name = ?", (name,)
        ) as cursor:
            if await cursor.fetchone() is not None:
                continue  # already applied

        sql = sql_file.read_text(encoding="utf-8")
        statements = [s.strip() for s in sql.split(";") if s.strip()]
        for stmt in statements:
            await db.execute(stmt)
        await db.execute("INSERT INTO schema_migrations (name) VALUES (?)", (name,))
        await db.commit()
