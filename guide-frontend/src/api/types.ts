// TypeScript interfaces mirroring all Rust backend models

// ==================== Enums ====================

export type GameSystem = 'dnd5e' | 'pathfinder2e' | 'custom';

export type CharacterType = 'pc' | 'npc' | 'monster';

export type Condition =
  | 'Blinded'
  | 'Charmed'
  | 'Deafened'
  | 'Exhausted'
  | 'Frightened'
  | 'Grappled'
  | 'Incapacitated'
  | 'Invisible'
  | 'Paralyzed'
  | 'Petrified'
  | 'Poisoned'
  | 'Prone'
  | 'Restrained'
  | 'Stunned'
  | 'Unconscious';

export const ALL_CONDITIONS: Condition[] = [
  'Blinded', 'Charmed', 'Deafened', 'Exhausted', 'Frightened',
  'Grappled', 'Incapacitated', 'Invisible', 'Paralyzed', 'Petrified',
  'Poisoned', 'Prone', 'Restrained', 'Stunned', 'Unconscious',
];

export type EncounterStatus = 'pending' | 'active' | 'completed';

export type EventType =
  | 'combat'
  | 'roleplay'
  | 'exploration'
  | 'skill_challenge'
  | 'item_found'
  | 'npc_introduced'
  | 'quest_update'
  | 'revelation'
  | 'other';

export const ALL_EVENT_TYPES: EventType[] = [
  'combat', 'roleplay', 'exploration', 'skill_challenge', 'item_found',
  'npc_introduced', 'quest_update', 'revelation', 'other',
];

export type EventSignificance = 'minor' | 'moderate' | 'major' | 'critical';

export type IngestionStatus = 'pending' | 'processing' | 'completed' | 'failed';

export type Perspective = 'dm' | 'player';

export type DocumentKind = 'campaign' | 'global';

export type GeneratedEncounterType = 'combat' | 'social' | 'exploration' | 'puzzle';

export type HookPriority = 'low' | 'medium' | 'high';

// ==================== Core Models ====================

export interface Campaign {
  id: string;
  name: string;
  description: string | null;
  game_system: GameSystem;
  world_state: WorldState | null;
  created_at: string;
  updated_at: string;
}

export interface WorldState {
  current_location: string | null;
  current_date_in_world: string | null;
  active_quests: string[];
  completed_quests: string[];
  custom_notes: string | null;
}

export interface AbilityScores {
  strength: number;
  dexterity: number;
  constitution: number;
  intelligence: number;
  wisdom: number;
  charisma: number;
}

export interface PlotHook {
  summary: string;
  priority: HookPriority;
  related_npcs: string[];
}

export interface Backstory {
  raw_text: string | null;
  hooks: PlotHook[];
  motivations: string[];
  key_relationships: string[];
  secrets: string[];
}

export interface Character {
  id: string;
  campaign_id: string;
  name: string;
  character_type: CharacterType;
  class: string | null;
  race: string | null;
  level: number;
  max_hp: number;
  current_hp: number;
  armor_class: number;
  speed: number;
  ability_scores: AbilityScores;
  conditions: Condition[];
  backstory: Backstory | null;
  is_alive: boolean;
  created_at: string;
  updated_at: string;
}

export interface ActionBudget {
  action: boolean;
  bonus_action: boolean;
  reaction: boolean;
  movement_remaining: number;
}

export interface CombatParticipant {
  id: string;
  encounter_id: string;
  character_id: string;
  name: string;
  initiative_roll: number;
  initiative_bonus: number;
  initiative_total: number;
  current_hp: number;
  max_hp: number;
  armor_class: number;
  conditions: Condition[];
  action_budget: ActionBudget;
  is_active: boolean;
}

export interface EncounterSummary {
  id: string;
  campaign_id: string;
  session_id: string | null;
  name: string;
  description: string | null;
  status: EncounterStatus;
  current_round: number;
  current_turn_index: number;
  participants: CombatParticipant[];
  created_at: string;
  updated_at: string;
}

export interface Session {
  id: string;
  campaign_id: string;
  session_number: number;
  title: string | null;
  status: 'pending' | 'started' | 'ended';
  started_at: string | null;
  ended_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface SessionEvent {
  id: string;
  session_id: string;
  event_type: EventType;
  description: string;
  significance: EventSignificance;
  is_player_visible: boolean;
  involved_character_ids: string[];
  created_at: string;
}

export interface SessionSummary {
  session_id: string;
  perspective: Perspective;
  content: string;
  generated_at: string;
}

export interface CampaignDocument {
  id: string;
  campaign_id: string;
  filename: string;
  file_size: number;
  status: IngestionStatus;
  uploaded_at: string;
  ingested_at: string | null;
}

export interface GlobalDocument {
  id: string;
  filename: string;
  file_size: number;
  status: IngestionStatus;
  uploaded_at: string;
  ingested_at: string | null;
}

export interface EnemySuggestion {
  name: string;
  count: number;
  challenge_rating: string;
  notes: string | null;
}

export interface GeneratedEncounter {
  title: string;
  encounter_type: GeneratedEncounterType;
  description: string;
  enemies: EnemySuggestion[];
  narrative_hook: string;
  terrain_features: string[];
  suggested_rewards: string[];
}

// ==================== Request Types ====================

export interface CreateCampaignRequest {
  name: string;
  description?: string;
  game_system: GameSystem;
}

export interface UpdateCampaignRequest {
  name?: string;
  description?: string;
  game_system?: GameSystem;
  world_state?: WorldState;
}

export interface CreateCharacterRequest {
  name: string;
  character_type: CharacterType;
  class?: string;
  race?: string;
  level?: number;
  max_hp: number;
  armor_class: number;
  speed?: number;
  ability_scores?: Partial<AbilityScores>;
  backstory_text?: string;
}

export interface UpdateCharacterRequest {
  name?: string;
  class?: string;
  race?: string;
  level?: number;
  max_hp?: number;
  current_hp?: number;
  armor_class?: number;
  speed?: number;
  ability_scores?: Partial<AbilityScores>;
  conditions?: Condition[];
  is_alive?: boolean;
}

export interface CreateSessionRequest {
  title?: string;
}

export interface CreateSessionEventRequest {
  event_type: EventType;
  description: string;
  significance: EventSignificance;
  is_player_visible: boolean;
  involved_character_ids?: string[];
}

export interface CreateEncounterRequest {
  session_id?: string;
  name: string;
  description?: string;
  participant_character_ids: string[];
}

export interface UpdateParticipantRequest {
  current_hp?: number;
  hp_delta?: number;
  conditions?: Condition[];
  action_budget?: Partial<ActionBudget>;
}

export interface GenerateRequest {
  context: string;
  party_level: number;
}

export interface ChatRequest {
  message: string;
  perspective: Perspective;
}

// ==================== Health ====================

export interface HealthResponse {
  status: string;
}

export interface VersionResponse {
  version: string;
  name: string;
}
