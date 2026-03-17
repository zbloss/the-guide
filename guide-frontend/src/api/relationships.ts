import { apiGet, apiPost, apiDelete } from './client';
import type { CharacterRelationship, CreateRelationshipRequest } from './types';

export function listRelationships(campaignId: string): Promise<CharacterRelationship[]> {
  return apiGet<CharacterRelationship[]>(`/campaigns/${campaignId}/relationships`);
}

export function createRelationship(campaignId: string, data: CreateRelationshipRequest): Promise<CharacterRelationship> {
  return apiPost<CharacterRelationship>(`/campaigns/${campaignId}/relationships`, data);
}

export function deleteRelationship(campaignId: string, relId: string): Promise<void> {
  return apiDelete(`/campaigns/${campaignId}/relationships/${relId}`);
}
