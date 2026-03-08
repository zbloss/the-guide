"""Port of the 11 Rust combat unit tests."""

from __future__ import annotations

from uuid import uuid4

import pytest

from guide.combat.engine import CombatEngine, build_participant
from guide.combat.initiative import InitiativeEntry, sort_initiative
from guide.errors import InvalidInputError
from guide.models.encounter import Encounter
from guide.models.shared import Condition, EncounterStatus


def _make_encounter(specs: list[tuple[str, int, int, int]]) -> Encounter:
    """specs: (name, initiative_roll, dex_mod, max_hp)"""
    enc_id = uuid4()
    participants = [
        build_participant(
            character_id=uuid4(),
            encounter_id=enc_id,
            name=name,
            initiative_roll=roll,
            initiative_modifier=dex_mod,
            max_hp=hp,
            current_hp=hp,
            armor_class=12,
            speed=30,
        )
        for name, roll, dex_mod, hp in specs
    ]
    from datetime import datetime, timezone

    now = datetime.now(timezone.utc)
    return Encounter(
        id=enc_id,
        session_id=uuid4(),
        campaign_id=uuid4(),
        name="Test Encounter",
        status=EncounterStatus.pending,
        round=0,
        current_turn_index=0,
        participants=participants,
        created_at=now,
        updated_at=now,
    )


def test_encounter_start_sorts_initiative():
    enc = _make_encounter(
        [
            ("Goblin A", 5, 1, 7),
            ("Fighter", 18, 2, 50),
            ("Rogue", 14, 3, 35),
        ]
    )
    engine = CombatEngine(enc)
    engine.start()

    assert engine.encounter.status == EncounterStatus.active
    assert engine.encounter.round == 1
    names = [p.name for p in engine.encounter.participants]
    # Fighter total=20, Rogue total=17, Goblin total=6
    assert names == ["Fighter", "Rogue", "Goblin A"]


def test_first_turn_is_highest_initiative():
    enc = _make_encounter([("Wizard", 3, -1, 20), ("Barbarian", 19, 2, 60)])
    engine = CombatEngine(enc)
    engine.start()
    assert engine.current_participant().name == "Barbarian"


def test_next_turn_advances_correctly():
    enc = _make_encounter([("A", 20, 0, 10), ("B", 15, 0, 10), ("C", 10, 0, 10)])
    engine = CombatEngine(enc)
    engine.start()

    assert engine.current_participant().name == "A"
    assert engine.next_turn().name == "B"
    assert engine.next_turn().name == "C"


def test_round_increments_on_wrap():
    enc = _make_encounter([("Solo", 10, 0, 30)])
    engine = CombatEngine(enc)
    engine.start()
    assert engine.encounter.round == 1

    engine.next_turn()
    assert engine.encounter.round == 2

    engine.next_turn()
    assert engine.encounter.round == 3


def test_hp_damage_and_defeat():
    enc = _make_encounter([("Goblin", 10, 0, 7)])
    engine = CombatEngine(enc)
    engine.start()

    goblin_id = engine.encounter.participants[0].id
    hp = engine.apply_hp_change(goblin_id, -5)
    assert hp == 2
    assert not engine.encounter.participants[0].is_defeated

    hp = engine.apply_hp_change(goblin_id, -10)
    assert hp == 0
    assert engine.encounter.participants[0].is_defeated
    assert Condition.unconscious in engine.encounter.participants[0].conditions


def test_healing_does_not_exceed_max_hp():
    enc = _make_encounter([("Paladin", 12, 1, 50)])
    engine = CombatEngine(enc)
    engine.start()

    pid = engine.encounter.participants[0].id
    engine.apply_hp_change(pid, -20)
    hp = engine.apply_hp_change(pid, 100)
    assert hp == 50


def test_set_hp_exact():
    enc = _make_encounter([("Cleric", 8, 0, 40)])
    engine = CombatEngine(enc)
    engine.start()

    pid = engine.encounter.participants[0].id
    hp = engine.set_hp(pid, 15)
    assert hp == 15


def test_condition_add_and_remove():
    enc = _make_encounter([("Rogue", 16, 3, 35)])
    engine = CombatEngine(enc)
    engine.start()

    pid = engine.encounter.participants[0].id
    engine.add_condition(pid, Condition.poisoned)
    assert Condition.poisoned in engine.encounter.participants[0].conditions

    engine.remove_condition(pid, Condition.poisoned)
    assert Condition.poisoned not in engine.encounter.participants[0].conditions


def test_cannot_start_active_encounter():
    enc = _make_encounter([("Fighter", 15, 2, 50)])
    engine = CombatEngine(enc)
    engine.start()

    with pytest.raises(InvalidInputError):
        engine.start()


def test_end_encounter():
    enc = _make_encounter([("Fighter", 15, 2, 50)])
    engine = CombatEngine(enc)
    engine.start()
    engine.end()

    assert engine.encounter.status == EncounterStatus.completed

    with pytest.raises(InvalidInputError):
        engine.end()


def test_initiative_sort_utility():
    entries = [
        InitiativeEntry(name="A", roll=5, modifier=2, total=7),
        InitiativeEntry(name="B", roll=15, modifier=0, total=15),
        InitiativeEntry(name="C", roll=10, modifier=-1, total=9),
    ]
    sorted_entries = sort_initiative(entries)
    assert sorted_entries[0].name == "B"  # total 15
    assert sorted_entries[1].name == "C"  # total 9
    assert sorted_entries[2].name == "A"  # total 7
