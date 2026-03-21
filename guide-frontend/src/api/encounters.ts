import { apiGet, apiPost, apiPut, apiDelete, BASE_URL } from './client';
import type { EncounterSummary, GeneratedEncounter, CreateEncounterRequest, UpdateParticipantRequest, GenerateRequest, EncounterTurnSnapshot } from './types';

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

export function getEncounterReplay(
  campaignId: string,
  encId: string,
): Promise<EncounterTurnSnapshot[]> {
  return new Promise((resolve, reject) => {
    const url = `${BASE_URL}/campaigns/${campaignId}/encounters/${encId}/replay`;
    const es = new EventSource(url);
    const snapshots: EncounterTurnSnapshot[] = [];

    es.addEventListener('snapshot', (e: MessageEvent) => {
      snapshots.push(JSON.parse(e.data) as EncounterTurnSnapshot);
    });

    es.addEventListener('done', () => {
      es.close();
      resolve(snapshots);
    });

    es.onerror = () => {
      es.close();
      reject(new Error('SSE connection failed'));
    };
  });
}
