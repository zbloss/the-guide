from __future__ import annotations

import random
from dataclasses import dataclass


@dataclass
class InitiativeEntry:
    name: str
    roll: int
    modifier: int
    total: int


def roll_d20() -> int:
    return random.randint(1, 20)


def roll_initiative(dex_modifier: int) -> InitiativeEntry:
    roll = roll_d20()
    return InitiativeEntry(name="", roll=roll, modifier=dex_modifier, total=roll + dex_modifier)


def sort_initiative(entries: list[InitiativeEntry]) -> list[InitiativeEntry]:
    """Sort descending by total, ties broken by modifier (higher wins)."""
    return sorted(entries, key=lambda e: (e.total, e.modifier), reverse=True)
