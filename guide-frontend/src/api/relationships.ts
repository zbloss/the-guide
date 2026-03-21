import { apiGet, apiPost, apiDelete, apiPatch } from './client';
import type { CharacterRelationship, CreateRelationshipRequest, UpdateRelationshipRequest } from './types';

export function listRelationships(campaignId: string): Promise<CharacterRelationship[]> {
  return apiGet<CharacterRelationship[]>(`/campaigns/${campaignId}/relationships`);
}

export function createRelationship(campaignId: string, data: CreateRelationshipRequest): Promise<CharacterRelationship> {
  return apiPost<CharacterRelationship>(`/campaigns/${campaignId}/relationships`, data);
}

export function updateRelationship(campaignId: string, relId: string, data: UpdateRelationshipRequest): Promise<CharacterRelationship> {
  return apiPatch<CharacterRelationship>(`/campaigns/${campaignId}/relationships/${relId}`, data);
}

export function deleteRelationship(campaignId: string, relId: string): Promise<void> {
  return apiDelete(`/campaigns/${campaignId}/relationships/${relId}`);
}
