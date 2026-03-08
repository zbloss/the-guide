use guide_combat::{build_participant, initiative::sort_initiative, CombatEngine};
use guide_core::models::{Condition, Encounter, EncounterStatus};
use uuid::Uuid;

fn make_encounter(participants: &[(&str, i32, i32, i32)]) -> Encounter {
    // (name, initiative_roll, dex_mod, max_hp)
    let enc_id = Uuid::new_v4();
    let parts = participants
        .iter()
        .map(|(name, roll, dex_mod, hp)| {
            build_participant(
                Uuid::new_v4(),
                enc_id,
                *name,
                *roll,
                *dex_mod,
                *hp,
                *hp,
                12,
                30,
            )
        })
        .collect();

    Encounter {
        id: enc_id,
        session_id: Uuid::new_v4(),
        campaign_id: Uuid::new_v4(),
        name: Some("Test Encounter".to_string()),
        description: None,
        status: EncounterStatus::Pending,
        round: 0,
        current_turn_index: 0,
        participants: parts,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

#[test]
fn test_encounter_start_sorts_initiative() {
    let enc = make_encounter(&[
        ("Goblin A", 5, 1, 7),
        ("Fighter", 18, 2, 50),
        ("Rogue", 14, 3, 35),
    ]);
    let mut engine = CombatEngine::new(enc);
    engine.start().expect("start failed");

    assert_eq!(engine.encounter.status, EncounterStatus::Active);
    assert_eq!(engine.encounter.round, 1);

    // Initiative totals: Fighter=20, Rogue=17, Goblin=6 → sorted descending
    let names: Vec<&str> = engine
        .encounter
        .participants
        .iter()
        .map(|p| p.name.as_str())
        .collect();
    assert_eq!(names, vec!["Fighter", "Rogue", "Goblin A"]);
}

#[test]
fn test_first_turn_is_highest_initiative() {
    let enc = make_encounter(&[
        ("Wizard", 3, -1, 20),
        ("Barbarian", 19, 2, 60),
    ]);
    let mut engine = CombatEngine::new(enc);
    engine.start().unwrap();

    let current = engine.current_participant().unwrap();
    assert_eq!(current.name, "Barbarian");
}

#[test]
fn test_next_turn_advances_correctly() {
    let enc = make_encounter(&[
        ("A", 20, 0, 10),
        ("B", 15, 0, 10),
        ("C", 10, 0, 10),
    ]);
    let mut engine = CombatEngine::new(enc);
    engine.start().unwrap();

    assert_eq!(engine.current_participant().unwrap().name, "A");

    let next = engine.next_turn().unwrap();
    assert_eq!(next.name, "B");

    let next = engine.next_turn().unwrap();
    assert_eq!(next.name, "C");
}

#[test]
fn test_round_increments_on_wrap() {
    let enc = make_encounter(&[("Solo", 10, 0, 30)]);
    let mut engine = CombatEngine::new(enc);
    engine.start().unwrap();
    assert_eq!(engine.encounter.round, 1);

    engine.next_turn().unwrap();
    assert_eq!(engine.encounter.round, 2);

    engine.next_turn().unwrap();
    assert_eq!(engine.encounter.round, 3);
}

#[test]
fn test_hp_damage_and_defeat() {
    let enc = make_encounter(&[("Goblin", 10, 0, 7)]);
    let mut engine = CombatEngine::new(enc);
    engine.start().unwrap();

    let goblin_id = engine.encounter.participants[0].id;

    let hp = engine.apply_hp_change(goblin_id, -5).unwrap();
    assert_eq!(hp, 2);
    assert!(!engine.encounter.participants[0].is_defeated);

    let hp = engine.apply_hp_change(goblin_id, -10).unwrap();
    assert_eq!(hp, 0);
    assert!(engine.encounter.participants[0].is_defeated);
    assert!(engine.encounter.participants[0]
        .conditions
        .contains(&Condition::Unconscious));
}

#[test]
fn test_healing_does_not_exceed_max_hp() {
    let enc = make_encounter(&[("Paladin", 12, 1, 50)]);
    let mut engine = CombatEngine::new(enc);
    engine.start().unwrap();

    let id = engine.encounter.participants[0].id;
    engine.apply_hp_change(id, -20).unwrap();

    let hp = engine.apply_hp_change(id, 100).unwrap();
    assert_eq!(hp, 50);
}

#[test]
fn test_set_hp_exact() {
    let enc = make_encounter(&[("Cleric", 8, 0, 40)]);
    let mut engine = CombatEngine::new(enc);
    engine.start().unwrap();

    let id = engine.encounter.participants[0].id;
    let hp = engine.set_hp(id, 15).unwrap();
    assert_eq!(hp, 15);
}

#[test]
fn test_condition_add_and_remove() {
    let enc = make_encounter(&[("Rogue", 16, 3, 35)]);
    let mut engine = CombatEngine::new(enc);
    engine.start().unwrap();

    let id = engine.encounter.participants[0].id;

    engine.add_condition(id, Condition::Poisoned).unwrap();
    assert!(engine.encounter.participants[0]
        .conditions
        .contains(&Condition::Poisoned));

    engine.remove_condition(id, &Condition::Poisoned).unwrap();
    assert!(!engine.encounter.participants[0]
        .conditions
        .contains(&Condition::Poisoned));
}

#[test]
fn test_cannot_start_active_encounter() {
    let enc = make_encounter(&[("Fighter", 15, 2, 50)]);
    let mut engine = CombatEngine::new(enc);
    engine.start().unwrap();

    let result = engine.start();
    assert!(result.is_err());
}

#[test]
fn test_end_encounter() {
    let enc = make_encounter(&[("Fighter", 15, 2, 50)]);
    let mut engine = CombatEngine::new(enc);
    engine.start().unwrap();
    engine.end().unwrap();

    assert_eq!(engine.encounter.status, EncounterStatus::Completed);
    assert!(engine.end().is_err());
}

#[test]
fn test_initiative_sort_utility() {
    let entries = vec![
        {
            let mut e = sort_initiative(vec![guide_combat::initiative::InitiativeEntry {
                name: "A".into(),
                roll: 5,
                modifier: 2,
                total: 7,
            }]);
            e.remove(0)
        },
        guide_combat::initiative::InitiativeEntry {
            name: "B".into(),
            roll: 15,
            modifier: 0,
            total: 15,
        },
        guide_combat::initiative::InitiativeEntry {
            name: "C".into(),
            roll: 10,
            modifier: -1,
            total: 9,
        },
    ];
    let sorted = sort_initiative(entries);

    assert_eq!(sorted[0].name, "B"); // 15
    assert_eq!(sorted[1].name, "C"); // 9
    assert_eq!(sorted[2].name, "A"); // 7
}
