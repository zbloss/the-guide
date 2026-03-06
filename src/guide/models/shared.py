from enum import Enum


class GameSystem(str, Enum):
    dnd5e = "dnd5e"
    pathfinder2e = "pathfinder2e"


class CharacterType(str, Enum):
    pc = "pc"
    npc = "npc"
    monster = "monster"


class Condition(str, Enum):
    blinded = "blinded"
    charmed = "charmed"
    deafened = "deafened"
    frightened = "frightened"
    grappled = "grappled"
    incapacitated = "incapacitated"
    invisible = "invisible"
    paralyzed = "paralyzed"
    petrified = "petrified"
    poisoned = "poisoned"
    prone = "prone"
    restrained = "restrained"
    stunned = "stunned"
    unconscious = "unconscious"


class EncounterStatus(str, Enum):
    pending = "pending"
    active = "active"
    completed = "completed"
    fled = "fled"


class LoreType(str, Enum):
    npc = "npc"
    location = "location"
    item = "item"
    plot = "plot"
    mechanic = "mechanic"
    backstory = "backstory"
    session_event = "session_event"


class EventType(str, Enum):
    combat = "combat"
    exploration = "exploration"
    social = "social"
    rest = "rest"
    level_up = "level_up"
    item_found = "item_found"
    npc_met = "npc_met"
    plot_revealed = "plot_revealed"
    custom = "custom"


class EventSignificance(str, Enum):
    minor = "minor"
    major = "major"
    milestone = "milestone"


class IngestionStatus(str, Enum):
    pending = "pending"
    processing = "processing"
    completed = "completed"
    failed = "failed"


class Perspective(str, Enum):
    dm = "dm"
    player = "player"
