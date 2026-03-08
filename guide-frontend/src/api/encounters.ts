import { apiGet, apiPost, apiPut, apiDelete } from './client';
import type { EncounterSummary, GeneratedEncounter, CreateEncounterRequest, UpdateParticipantRequest, GenerateRequest } from './types';

export function listEncounters(campaignId: string): Promise<EncounterSummary[]> {
  return apiGet<EncounterSummary[]>(`/campaigns/${campaignId}/encounters`);
}

export function createEncounter(campaignId: string, data: CreateEncounterRequest): Promise<EncounterSummary> {
  return apiPost<EncounterSummary>(`/campaigns/${campaignId}/encounters`, data);
}

export function getEncounter(campaignId: string, encId: string): Promise<EncounterSummary> {
  return apiGet<EncounterSummary>(`/campaigns/${campaignId}/encounters/${encId}`);
}

export function deleteEncounter(campaignId: string, encId: string): Promise<void> {
  return apiDelete(`/campaigns/${campaignId}/encounters/${encId}`);
}

export function startEncounter(campaignId: string, encId: string): Promise<EncounterSummary> {
  return apiPost<EncounterSummary>(`/campaigns/${campaignId}/encounters/${encId}/start`);
}

export function nextTurn(campaignId: string, encId: string): Promise<EncounterSummary> {
  return apiPost<EncounterSummary>(`/campaigns/${campaignId}/encounters/${encId}/next-turn`);
}

export function endEncounter(campaignId: string, encId: string): Promise<EncounterSummary> {
  return apiPost<EncounterSummary>(`/campaigns/${campaignId}/encounters/${encId}/end`);
}

export function updateParticipant(campaignId: string, encId: string, participantId: string, data: UpdateParticipantRequest): Promise<EncounterSummary> {
  return apiPut<EncounterSummary>(`/campaigns/${campaignId}/encounters/${encId}/participants/${participantId}`, data);
}

export function generateEncounter(campaignId: string, data: GenerateRequest): Promise<GeneratedEncounter> {
  return apiPost<GeneratedEncounter>(`/campaigns/${campaignId}/encounters/generate`, data);
}
