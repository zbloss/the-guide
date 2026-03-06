use rand::Rng;

/// A participant with their initiative entry (before creating a full CombatParticipant).
#[derive(Debug, Clone)]
pub struct InitiativeEntry {
    pub name: String,
    pub roll: i32,
    pub modifier: i32,
    pub total: i32,
}

/// Roll a d20 and return the result.
pub fn roll_d20() -> i32 {
    rand::thread_rng().gen_range(1..=20)
}

/// Roll initiative for a participant given their DEX modifier.
pub fn roll_initiative(dex_modifier: i32) -> InitiativeEntry {
    let roll = roll_d20();
    InitiativeEntry {
        name: String::new(),
        roll,
        modifier: dex_modifier,
        total: roll + dex_modifier,
    }
}

/// Sort a list of initiative entries by total descending.
/// Ties are broken by modifier (higher wins), then randomly.
pub fn sort_initiative(mut entries: Vec<InitiativeEntry>) -> Vec<InitiativeEntry> {
    entries.sort_by(|a, b| {
        b.total
            .cmp(&a.total)
            .then_with(|| b.modifier.cmp(&a.modifier))
    });
    entries
}
