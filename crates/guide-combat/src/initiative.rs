use rand::Rng;

#[derive(Debug, Clone)]
pub struct InitiativeEntry {
    pub name: String,
    pub roll: i32,
    pub modifier: i32,
    pub total: i32,
}

pub fn roll_d20() -> i32 {
    rand::rng().random_range(1..=20)
}

pub fn roll_initiative(dex_modifier: i32) -> InitiativeEntry {
    let roll = roll_d20();
    InitiativeEntry {
        name: String::new(),
        roll,
        modifier: dex_modifier,
        total: roll + dex_modifier,
    }
}

/// Sort entries by total DESC, then modifier DESC, then id string (not applicable here).
pub fn sort_initiative(mut entries: Vec<InitiativeEntry>) -> Vec<InitiativeEntry> {
    entries.sort_by(|a, b| {
        b.total
            .cmp(&a.total)
            .then_with(|| b.modifier.cmp(&a.modifier))
    });
    entries
}
