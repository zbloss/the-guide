// TypeScript interfaces mirroring all Rust backend models

// ==================== Enums ====================

export type GameSystem = 'dnd5e' | 'pathfinder2e' | 'custom';

export type CharacterType = 'pc' | 'npc' | 'monster';

export type Condition =
  | 'blinded'
  | 'charmed'
  | 'deafened'
  | 'frightened'
  | 'grappled'
  | 'incapacitated'
  | 'invisible'
  | 'paralyzed'
  | 'petrified'
  | 'poisoned'
  | 'prone'
  | 'restrained'
  | 'stunned'
  | 'unconscious';

export const ALL_CONDITIONS: Condition[] = [
  'blinded', 'charmed', 'deafened', 'frightened',
  'grappled', 'incapacitated', 'invisible', 'paralyzed', 'petrified',
  'poisoned', 'prone', 'restrained', 'stunned', 'unconscious',
];

export type EncounterStatus = 'pending' | 'active' | 'completed';

export type EventType =
  | 'combat'
  | 'exploration'
  | 'social'
  | 'rest'
  | 'level_up'
  | 'item_found'
  | 'npc_met'
  | 'plot_revealed'
  | 'custom';

export const ALL_EVENT_TYPES: EventType[] = [
  'combat', 'exploration', 'social', 'rest', 'level_up',
  'item_found', 'npc_met', 'plot_revealed', 'custom',
];

export type EventSignificance = 'minor' | 'major' | 'milestone';

export type IngestionStatus = 'pending' | 'processing' | 'completed' | 'failed';

export type Perspective = 'dm' | 'player';

export type DocumentKind = 'campaign' | 'global';

export type GeneratedEncounterType = 'combat' | 'social' | 'exploration' | 'puzzle' | 'mixed';

export type HookPriority = 'low' | 'medium' | 'high' | 'critical';

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
  id: string;
  character_id: string;
  description: string;
  priority: HookPriority;
  is_active: boolean;
  llm_extracted: boolean;
}

export interface Backstory {
  raw_text: string;
  extracted_hooks: PlotHook[];
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
  has_action: boolean;
  has_bonus_action: boolean;
  has_reaction: boolean;
  movement_remaining: number;
}

export interface CombatParticipant {
  id: string;
  encounter_id: string;
  character_id: string;
  name: string;
  initiative_roll: number;
  initiative_modifier: number;
  initiative_total: number;
  current_hp: number;
  max_hp: number;
  armor_class: number;
  conditions: Condition[];
  action_budget: ActionBudget;
  is_defeated: boolean;
}

export interface EncounterSummary {
  id: string;
  campaign_id: string;
  session_id: string | null;
  name: string | null;
  description: string | null;
  status: EncounterStatus;
  round: number;
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
  started_at: string | null;
  ended_at: string | null;
  created_at: string;
  updated_at: string;
}

export function deriveSessionStatus(s: Session): 'pending' | 'started' | 'ended' {
  if (s.ended_at) return 'ended';
  if (s.started_at) return 'started';
  return 'pending';
}

export interface SessionEvent {
  id: string;
  session_id: string;
  event_type: EventType;
  description: string;
  significance: EventSignificance;
  is_player_visible: boolean;
  involved_character_ids: string[];
  occurred_at: string;
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
  file_size_bytes: number;
  ingestion_status: IngestionStatus;
  uploaded_at: string;
  ingested_at: string | null;
}

export interface GlobalDocument {
  id: string;
  filename: string;
  file_size_bytes: number;
  ingestion_status: IngestionStatus;
  uploaded_at: string;
  ingested_at: string | null;
}

export interface EnemySuggestion {
  name: string;
  count: number;
  cr: number | null;
}

export interface GeneratedEncounter {
  title: string;
  encounter_type: GeneratedEncounterType;
  description: string;
  suggested_enemies: EnemySuggestion[];
  narrative_hook: string;
  alternative: string | null;
  challenge_rating: number | null;
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
  backstory_text?: string;
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
  name?: string;
  description?: string;
  participant_character_ids: string[];
}

export interface UpdateParticipantRequest {
  name?: string;
  hp_delta?: number;
  set_hp?: number;
  add_condition?: Condition;
  remove_condition?: Condition;
  spend_action?: boolean;
  spend_bonus_action?: boolean;
  spend_reaction?: boolean;
  spend_movement?: number;
}

// ==================== Playstyle Profile ====================

export type Pacing = 'fast' | 'moderate' | 'slow';
export type Lethality = 'lethal' | 'moderate' | 'forgiving';
export type TonePreference = 'dark' | 'balanced' | 'heroic';
export type CombatComplexity = 'high' | 'moderate' | 'simple';

export interface PlaystyleProfile {
  pacing: Pacing;
  lethality: Lethality;
  tone: TonePreference;
  combat_complexity: CombatComplexity;
  roleplay_focus: number; // 1-10
  exploration_focus: number; // 1-10
  custom_notes: string;
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
