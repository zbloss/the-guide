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

export type Perspective = 'dm';

export type DocumentKind = 'dm_guide' | 'monster_manual' | 'srd' | 'campaign' | 'supplemental';

export type GeneratedEncounterType = 'combat' | 'social' | 'exploration' | 'puzzle' | 'mixed';

export type HookPriority = 'low' | 'medium' | 'high' | 'critical';

// ==================== Core Models ====================

export interface Campaign {
  id: string;
  name: string;
  description: string | null;
  game_system: GameSystem;
  world_state: WorldState | null;
  share_token: string | null;
  current_chapter?: string | null;
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

export interface SpellSlot {
  level: number;
  total: number;
  remaining: number;
}

export interface SpendSlotRequest {
  level: number;
}

export interface RestoreSlotRequest {
  level?: number;
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
  spell_slots: SpellSlot[];
  backstory: Backstory | null;
  portrait_url: string | null;
  is_alive: boolean;
  created_at: string;
  updated_at: string;
}

export interface ConditionEntry {
  condition: Condition;
  duration_rounds: number | null;
  applied_round: number | null;
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
  conditions: ConditionEntry[];
  action_budget: ActionBudget;
  is_defeated: boolean;
  death_saves_success: number;
  death_saves_failure: number;
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
  map_url: string | null;
  created_at: string;
  updated_at: string;
}

// ==================== Encounter Templates ====================

export interface TemplateParticipant {
  name: string;
  max_hp: number;
  armor_class: number;
  speed: number;
}

export interface EncounterTemplate {
  id: string;
  name: string;
  description: string | null;
  participants: TemplateParticipant[];
  created_at: string;
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
  description: string | null;
  story_extraction_status: string;
  story_extraction_error: string | null;
}

export interface GlobalDocument {
  id: string;
  filename: string;
  file_size_bytes: number;
  ingestion_status: IngestionStatus;
  uploaded_at: string;
  ingested_at: string | null;
  document_kind: DocumentKind;
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
  current_chapter?: string;
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
  spell_slots?: SpellSlot[];
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
  spell_slots?: SpellSlot[];
}

export interface GenerateNpcRequest {
  prompt: string;
}

export interface EncounterDifficulty {
  easy_threshold: number;
  medium_threshold: number;
  hard_threshold: number;
  deadly_threshold: number;
  party_size: number;
  average_level: number;
  rating: string;
}

export interface ConsistencyIssue {
  category: string;
  description: string;
  severity: 'minor' | 'major';
}

export interface ConsistencyReport {
  campaign_id: string;
  issues: ConsistencyIssue[];
  summary: string;
  generated_at: string;
}

export interface SearchResultItem {
  id: string;
  type: 'character' | 'session' | 'event';
  label: string;
  session_id?: string;
}

export interface SearchResults {
  query: string;
  characters: SearchResultItem[];
  sessions: SearchResultItem[];
  events: SearchResultItem[];
}

export interface AtmosphereResponse {
  weather: string;
  ambient_sounds: string;
  sensory_details: string;
  full_description: string;
}

// ==================== Calendar ====================

export interface CalendarEntry {
  id: string;
  campaign_id: string;
  session_id: string | null;
  in_game_date: string;
  real_date: string;
  notes: string | null;
  created_at: string;
}

export interface CreateCalendarEntryRequest {
  session_id?: string;
  in_game_date: string;
  real_date: string;
  notes?: string;
}

// ==================== Loot ====================

export type LootItemType = 'weapon' | 'armor' | 'magic' | 'currency' | 'misc';

export interface LootItem {
  id: string;
  session_id: string;
  campaign_id: string;
  name: string;
  item_type: string;
  quantity: number;
  value_gp: number;
  assigned_to_char_id: string | null;
  notes: string | null;
  created_at: string;
}

export interface CreateLootItemRequest {
  name: string;
  item_type?: LootItemType;
  quantity?: number;
  value_gp?: number;
  assigned_to_char_id?: string;
  notes?: string;
}

// ==================== Homebrew ====================

export interface HomebrewRule {
  id: string;
  campaign_id: string;
  title: string;
  description: string;
  category: string;
  created_at: string;
}

export interface CreateHomebrewRuleRequest {
  title: string;
  description: string;
  category?: string;
}

// ==================== Factions ====================

export interface Faction {
  id: string;
  campaign_id: string;
  name: string;
  description: string | null;
  standing: string;
  notes: string | null;
  created_at: string;
}

export interface CreateFactionRequest {
  name: string;
  description?: string;
}

export interface UpdateFactionReputationRequest {
  standing: string;
  notes?: string;
}

// ==================== Chat History ====================

export interface ChatMessage {
  id: string;
  campaign_id: string;
  role: 'user' | 'assistant';
  content: string;
  perspective: string;
  created_at: string;
}

export interface ImprovPromptResponse {
  options: string[];
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
  add_condition?: ConditionEntry;
  remove_condition?: Condition;
  spend_action?: boolean;
  spend_bonus_action?: boolean;
  spend_reaction?: boolean;
  spend_movement?: number;
  add_death_save_success?: boolean;
  add_death_save_failure?: boolean;
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

// ==================== Homebrew Rules ====================

export interface HomebrewRule {
  id: string;
  campaign_id: string;
  title: string;
  description: string;
  category: string;
  created_at: string;
}

export interface CreateHomebrewRuleRequest {
  title: string;
  description: string;
  category?: string;
}

// ==================== Factions ====================

export interface Faction {
  id: string;
  campaign_id: string;
  name: string;
  description: string | null;
  standing: string;
  notes: string | null;
  created_at: string;
}

export interface CreateFactionRequest {
  name: string;
  description?: string;
}

export interface UpdateFactionReputationRequest {
  standing: string;
  notes?: string;
}

// ==================== Plot Hook Tracker ====================

export type PlotHookStatus = 'open' | 'active' | 'resolved';

export interface TrackedPlotHook {
  id: string;
  character_id: string;
  hook_text: string;
  status: PlotHookStatus;
  session_resolved_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateTrackedPlotHookRequest {
  hook_text: string;
  status?: PlotHookStatus;
}

export interface UpdateTrackedPlotHookRequest {
  status?: PlotHookStatus;
  session_resolved_id?: string;
}

// ==================== Webhooks ====================

export interface CampaignWebhook {
  id: string;
  campaign_id: string;
  url: string;
  events: string[];
  created_at: string;
}

export interface CreateWebhookRequest {
  url: string;
  events?: string[];
}

// ==================== Analytics ====================

export interface SessionsByMonth { month: string; count: number; }
export interface EncounterDifficultyEntry { difficulty: string; count: number; }
export interface CampaignAnalytics {
  sessions_count: number;
  encounters_count: number;
  characters_count: number;
  sessions_by_month: SessionsByMonth[];
  encounter_difficulty: EncounterDifficultyEntry[];
}

// ==================== Documents / Rules Search ====================

export interface RankedChunk {
  content: string;
  section_path: string;
  doc_title: string;
  score: number;
}

// ==================== Health ====================

export interface HealthResponse {
  status: string;
}

export interface VersionResponse {
  version: string;
  name: string;
}

// ==================== DM Prep ====================

export type PrepType = 'session_recap' | 'story_so_far' | 'story_ahead' | 'character_roadmap';

export interface DmPrepResult {
  id: string;
  campaign_id: string;
  prep_type: PrepType;
  content: string;
  character_id: string | null;
  generated_at: string;
}

export interface SessionRecapRequest {
  force_regenerate?: boolean;
}

export interface StoryContextRequest {
  current_chapter?: string;
  force_regenerate?: boolean;
}

export interface CharacterRoadmapRequest {
  current_chapter?: string;
  force_regenerate?: boolean;
}

export interface ParsedSheetResult {
  name: string;
  class: string | null;
  race: string | null;
  level: number;
  max_hp: number;
  armor_class: number;
  speed: number;
  ability_scores: AbilityScores;
  backstory_text: string | null;
  raw_extracted_text: string;
  parse_confidence: number;
}

// ==================== Relationships ====================

export const RELATIONSHIP_TYPES = [
  'ally', 'enemy', 'rival', 'mentor', 'student', 'family', 'lover',
  'employer', 'employee', 'friend', 'nemesis', 'neutral', 'unknown',
] as const;

export type RelationshipType = typeof RELATIONSHIP_TYPES[number];

export interface CharacterRelationship {
  id: string;
  campaign_id: string;
  from_character_id: string;
  to_character_id: string;
  relationship_type: string;
  notes: string | null;
  created_at: string;
}

export interface CreateRelationshipRequest {
  from_character_id: string;
  to_character_id: string;
  relationship_type: string;
  notes?: string;
}

// ==================== Encounter Replay ====================

export interface EncounterTurnSnapshot {
  id: string;
  encounter_id: string;
  turn_number: number;
  round_number: number;
  snapshot: EncounterSummary;
  recorded_at: string;
}

// ==================== Voice Transcription ====================

export interface TranscribeResponse {
  transcript: string;
}

// ==================== Story ====================

export type ArcStatus = 'open' | 'resolved' | 'abandoned';
export type StoryEventType = 'combat' | 'social' | 'revelation' | 'travel' | 'rest';
export type StorySignificance = 'major' | 'minor';
export type SubplotStatus = 'open' | 'resolved' | 'abandoned';

export interface ArcPoint {
  description: string;
  order: number;
}

export interface MonsterHint {
  name: string;
  count: number | null;
  cr: string | null;
}

export interface StoryArc {
  id: string;
  campaign_id: string;
  source_doc_id: string;
  title: string;
  description: string;
  arc_order: number;
  status: ArcStatus;
  dm_notes: string | null;
  created_at: string;
  updated_at: string;
}

export interface StoryEvent {
  id: string;
  campaign_id: string;
  arc_id: string | null;
  source_doc_id: string;
  title: string;
  description: string;
  event_type: StoryEventType;
  significance: StorySignificance;
  location: string | null;
  involved_characters: string[];
  event_order: number;
  is_dm_only: boolean;
  dm_notes: string | null;
  created_at: string;
  updated_at: string;
}

export interface StorySubplot {
  id: string;
  campaign_id: string;
  arc_id: string | null;
  source_doc_id: string;
  title: string;
  description: string;
  status: SubplotStatus;
  dm_notes: string | null;
  created_at: string;
  updated_at: string;
}

export interface CharacterArc {
  id: string;
  campaign_id: string;
  character_name: string;
  character_id: string | null;
  source_doc_id: string;
  description: string;
  arc_points: ArcPoint[];
  dm_notes: string | null;
  created_at: string;
  updated_at: string;
}

export interface PrepopulatedEncounter {
  id: string;
  campaign_id: string;
  story_event_id: string | null;
  source_doc_id: string;
  name: string;
  description: string;
  location: string | null;
  difficulty_hint: string | null;
  monsters: MonsterHint[];
  dm_notes: string | null;
  created_at: string;
  updated_at: string;
}

export interface StoryExtractionStatus {
  status: string;
  error: string | null;
}
