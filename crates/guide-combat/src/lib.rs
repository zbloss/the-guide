pub mod initiative;

pub use initiative::{roll_initiative, sort_initiative, InitiativeEntry};

use chrono::Utc;
use guide_core::{
    models::{ActionBudget, CombatParticipant, Condition, ConditionEntry, Encounter, EncounterStatus},
    GuideError, Result,
};
use rand::Rng;
use uuid::Uuid;

pub struct CombatEngine {
    pub encounter: Encounter,
}

impl CombatEngine {
    pub fn new(encounter: Encounter) -> Self {
        Self { encounter }
    }

    /// Roll initiative for all participants (if not already set) and sort them.
    /// Moves the encounter to `Active` status.
    pub fn start(&mut self) -> Result<()> {
        if self.encounter.status != EncounterStatus::Pending {
            return Err(GuideError::InvalidInput(
                "Encounter has already started".into(),
            ));
        }

        // Auto-roll initiative for any participant who has not had a roll set (roll == 0)
        let mut rng = rand::rng();
        for participant in &mut self.encounter.participants {
            if participant.initiative_roll == 0 {
                let roll: i32 = rng.random_range(1..=20);
                participant.initiative_roll = roll;
                participant.initiative_total = roll + participant.initiative_modifier;
            }
        }

        // Sort participants by initiative_total DESC, modifier DESC, then id ASC for tiebreak
        self.encounter.participants.sort_by(|a, b| {
            b.initiative_total
                .cmp(&a.initiative_total)
                .then_with(|| b.initiative_modifier.cmp(&a.initiative_modifier))
                .then_with(|| a.id.cmp(&b.id))
        });

        self.encounter.status = EncounterStatus::Active;
        self.encounter.round = 1;
        self.encounter.current_turn_index = 0;
        self.encounter.updated_at = Utc::now();

        Ok(())
    }

    /// Advance to the next participant's turn.
    /// Wraps to the next round when the last participant has gone.
    pub fn next_turn(&mut self) -> Result<&CombatParticipant> {
        if self.encounter.status != EncounterStatus::Active {
            return Err(GuideError::InvalidInput("Encounter is not active".into()));
        }

        let n = self.encounter.participants.len();
        if n == 0 {
            return Err(GuideError::InvalidInput("No participants in encounter".into()));
        }

        let current_idx = self.encounter.current_turn_index as usize;
        if let Some(p) = self.encounter.participants.get_mut(current_idx) {
            p.has_taken_turn = true;
        }

        let next_idx = (current_idx + 1) % n;
        self.encounter.current_turn_index = next_idx as i32;

        if next_idx == 0 {
            self.encounter.round += 1;
            let current_round = self.encounter.round;
            for p in &mut self.encounter.participants {
                p.has_taken_turn = false;
                p.action_budget.reset(30);
                // Auto-expire timed conditions: remove when applied_round + duration_rounds <= current_round
                p.conditions.retain(|e| {
                    match (e.applied_round, e.duration_rounds) {
                        (Some(applied), Some(dur)) => applied + dur > current_round,
                        _ => true, // permanent (no duration) or missing data — keep
                    }
                });
            }
        }

        self.encounter.updated_at = Utc::now();
        Ok(&self.encounter.participants[next_idx])
    }

    pub fn apply_hp_change(&mut self, participant_id: Uuid, delta: i32) -> Result<i32> {
        let participant = self
            .encounter
            .participants
            .iter_mut()
            .find(|p| p.id == participant_id)
            .ok_or_else(|| GuideError::NotFound(format!("Participant {participant_id}")))?;

        participant.current_hp = (participant.current_hp + delta).clamp(0, participant.max_hp);

        if participant.current_hp == 0 {
            participant.is_defeated = true;
            if !participant.conditions.iter().any(|e| e.condition == Condition::Unconscious) {
                participant.conditions.push(ConditionEntry {
                    condition: Condition::Unconscious,
                    duration_rounds: None,
                    applied_round: Some(self.encounter.round),
                });
            }
        }

        self.encounter.updated_at = Utc::now();
        Ok(participant.current_hp)
    }

    pub fn set_hp(&mut self, participant_id: Uuid, hp: i32) -> Result<i32> {
        let current = self
            .encounter
            .participants
            .iter()
            .find(|p| p.id == participant_id)
            .map(|p| p.current_hp)
            .ok_or_else(|| GuideError::NotFound(format!("Participant {participant_id}")))?;

        self.apply_hp_change(participant_id, hp - current)
    }

    pub fn add_condition(&mut self, participant_id: Uuid, mut entry: ConditionEntry) -> Result<()> {
        // Set applied_round from current encounter round
        entry.applied_round = Some(self.encounter.round);

        let participant = self
            .encounter
            .participants
            .iter_mut()
            .find(|p| p.id == participant_id)
            .ok_or_else(|| GuideError::NotFound(format!("Participant {participant_id}")))?;

        if !participant.conditions.iter().any(|e| e.condition == entry.condition) {
            participant.conditions.push(entry);
        }
        self.encounter.updated_at = Utc::now();
        Ok(())
    }

    pub fn remove_condition(&mut self, participant_id: Uuid, condition: &Condition) -> Result<()> {
        let participant = self
            .encounter
            .participants
            .iter_mut()
            .find(|p| p.id == participant_id)
            .ok_or_else(|| GuideError::NotFound(format!("Participant {participant_id}")))?;

        participant.conditions.retain(|e| &e.condition != condition);
        self.encounter.updated_at = Utc::now();
        Ok(())
    }

    pub fn end(&mut self) -> Result<()> {
        if self.encounter.status == EncounterStatus::Completed {
            return Err(GuideError::InvalidInput("Encounter already ended".into()));
        }
        self.encounter.status = EncounterStatus::Completed;
        self.encounter.updated_at = Utc::now();
        Ok(())
    }

    pub fn current_participant(&self) -> Option<&CombatParticipant> {
        self.encounter
            .participants
            .get(self.encounter.current_turn_index as usize)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_participant(
    character_id: Uuid,
    encounter_id: Uuid,
    name: impl Into<String>,
    initiative_roll: i32,
    initiative_modifier: i32,
    max_hp: i32,
    current_hp: i32,
    armor_class: i32,
    speed: i32,
) -> CombatParticipant {
    CombatParticipant {
        id: Uuid::new_v4(),
        encounter_id,
        character_id,
        name: name.into(),
        initiative_roll,
        initiative_modifier,
        initiative_total: initiative_roll + initiative_modifier,
        current_hp,
        max_hp,
        armor_class,
        conditions: Vec::new(),
        action_budget: ActionBudget::new(speed),
        has_taken_turn: false,
        is_defeated: false,
    }
}
