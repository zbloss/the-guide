import { apiGet, apiPost, apiPut, apiDelete } from './client';
import type { Faction, CreateFactionRequest, UpdateFactionReputationRequest } from './types';

export function listFactions(campaignId: string): Promise<Faction[]> {
  return apiGet<Faction[]>(`/campaigns/${campaignId}/factions`);
}

export function createFaction(campaignId: string, data: CreateFactionRequest): Promise<Faction> {
  return apiPost<Faction>(`/campaigns/${campaignId}/factions`, data);
}

export function updateFactionReputation(
  campaignId: string,
  factionId: string,
  data: UpdateFactionReputationRequest,
): Promise<Faction> {
  return apiPut<Faction>(`/campaigns/${campaignId}/factions/${factionId}/reputation`, data);
}

export function deleteFaction(campaignId: string, factionId: string): Promise<void> {
  return apiDelete(`/campaigns/${campaignId}/factions/${factionId}`);
}
