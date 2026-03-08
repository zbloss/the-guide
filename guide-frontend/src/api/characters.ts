import { apiGet, apiPost, apiPut, apiDelete } from './client';
import type { Character, Backstory, CreateCharacterRequest, UpdateCharacterRequest } from './types';

export function listCharacters(campaignId: string): Promise<Character[]> {
  return apiGet<Character[]>(`/campaigns/${campaignId}/characters`);
}

export function createCharacter(campaignId: string, data: CreateCharacterRequest): Promise<Character> {
  return apiPost<Character>(`/campaigns/${campaignId}/characters`, data);
}

export function getCharacter(campaignId: string, charId: string): Promise<Character> {
  return apiGet<Character>(`/campaigns/${campaignId}/characters/${charId}`);
}

export function updateCharacter(campaignId: string, charId: string, data: UpdateCharacterRequest): Promise<Character> {
  return apiPut<Character>(`/campaigns/${campaignId}/characters/${charId}`, data);
}

export function deleteCharacter(campaignId: string, charId: string): Promise<void> {
  return apiDelete(`/campaigns/${campaignId}/characters/${charId}`);
}

export function analyzeBackstory(campaignId: string, charId: string): Promise<Backstory> {
  return apiPost<Backstory>(`/campaigns/${campaignId}/characters/${charId}/analyze-backstory`);
}
