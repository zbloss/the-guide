import { apiGet, apiPost } from './client';
import type { DmPrepResult, SessionRecapRequest, StoryContextRequest, CharacterRoadmapRequest } from './types';

export function listPrepResults(campaignId: string): Promise<DmPrepResult[]> {
  return apiGet<DmPrepResult[]>(`/campaigns/${campaignId}/prep`);
}

export function generateSessionRecap(campaignId: string, req?: SessionRecapRequest): Promise<DmPrepResult> {
  return apiPost<DmPrepResult>(`/campaigns/${campaignId}/prep/session-recap`, req ?? {});
}

export function generateStorySoFar(campaignId: string, req?: StoryContextRequest): Promise<DmPrepResult> {
  return apiPost<DmPrepResult>(`/campaigns/${campaignId}/prep/story-so-far`, req ?? {});
}

export function generateStoryAhead(campaignId: string, req?: StoryContextRequest): Promise<DmPrepResult> {
  return apiPost<DmPrepResult>(`/campaigns/${campaignId}/prep/story-ahead`, req ?? {});
}

export function generateCharacterRoadmap(campaignId: string, charId: string, req?: CharacterRoadmapRequest): Promise<DmPrepResult> {
  return apiPost<DmPrepResult>(`/campaigns/${campaignId}/prep/character-roadmap/${charId}`, req ?? {});
}
