import { apiGet, apiPatch, apiPost } from './client';
import type { StoryArc, StoryEvent, StorySubplot, CharacterArc, PrepopulatedEncounter, StoryExtractionStatus } from './types';

// Story Arcs
export function listArcs(campaignId: string): Promise<StoryArc[]> {
  return apiGet<StoryArc[]>(`/campaigns/${campaignId}/story/arcs`);
}

export function updateArcNotes(campaignId: string, arcId: string, dm_notes: string | null): Promise<void> {
  return apiPatch<void>(`/campaigns/${campaignId}/story/arcs/${arcId}/notes`, { dm_notes });
}

export function updateArcStatus(campaignId: string, arcId: string, status: string): Promise<void> {
  return apiPatch<void>(`/campaigns/${campaignId}/story/arcs/${arcId}/status`, { status });
}

// Story Events
export function listEvents(campaignId: string, arcId?: string): Promise<StoryEvent[]> {
  const query = arcId ? `?arc_id=${encodeURIComponent(arcId)}` : '';
  return apiGet<StoryEvent[]>(`/campaigns/${campaignId}/story/events${query}`);
}

export function updateEventNotes(campaignId: string, eventId: string, dm_notes: string | null): Promise<void> {
  return apiPatch<void>(`/campaigns/${campaignId}/story/events/${eventId}/notes`, { dm_notes });
}

// Subplots
export function listSubplots(campaignId: string): Promise<StorySubplot[]> {
  return apiGet<StorySubplot[]>(`/campaigns/${campaignId}/story/subplots`);
}

export function updateSubplotNotes(campaignId: string, subplotId: string, dm_notes: string | null): Promise<void> {
  return apiPatch<void>(`/campaigns/${campaignId}/story/subplots/${subplotId}/notes`, { dm_notes });
}

export function updateSubplotStatus(campaignId: string, subplotId: string, status: string): Promise<void> {
  return apiPatch<void>(`/campaigns/${campaignId}/story/subplots/${subplotId}/status`, { status });
}

// Character Arcs
export function listCharacterArcs(campaignId: string): Promise<CharacterArc[]> {
  return apiGet<CharacterArc[]>(`/campaigns/${campaignId}/story/character-arcs`);
}

export function updateCharacterArcNotes(campaignId: string, arcId: string, dm_notes: string | null): Promise<void> {
  return apiPatch<void>(`/campaigns/${campaignId}/story/character-arcs/${arcId}/notes`, { dm_notes });
}

// Prepopulated Encounters
export function listStoryEncounters(campaignId: string): Promise<PrepopulatedEncounter[]> {
  return apiGet<PrepopulatedEncounter[]>(`/campaigns/${campaignId}/story/encounters`);
}

export function updateStoryEncounterNotes(campaignId: string, encId: string, dm_notes: string | null): Promise<void> {
  return apiPatch<void>(`/campaigns/${campaignId}/story/encounters/${encId}/notes`, { dm_notes });
}

export function activateEncounter(campaignId: string, encId: string): Promise<{ encounter_id: string }> {
  return apiPost<{ encounter_id: string }>(`/campaigns/${campaignId}/story/encounters/${encId}/activate`);
}

// Story extraction status
export function getStoryExtractionStatus(campaignId: string, docId: string): Promise<StoryExtractionStatus> {
  return apiGet<StoryExtractionStatus>(`/campaigns/${campaignId}/documents/${docId}/story-extraction-status`);
}

// Associate global doc with campaign
export function associateGlobalDoc(campaignId: string, globalDocId: string): Promise<void> {
  return apiPost<void>(`/campaigns/${campaignId}/global-docs`, { global_doc_id: globalDocId });
}
