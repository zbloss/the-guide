import { apiGet, apiPost, apiPut, apiDelete, BASE_URL } from './client';
import type { Character, Backstory, CreateCharacterRequest, UpdateCharacterRequest, GenerateNpcRequest } from './types';

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

export function generateNpc(campaignId: string, req: GenerateNpcRequest): Promise<Character> {
  return apiPost<Character>(`/campaigns/${campaignId}/npcs/generate`, req);
}

export function spendSpellSlot(campaignId: string, charId: string, level: number): Promise<Character> {
  return apiPost<Character>(`/campaigns/${campaignId}/characters/${charId}/spell-slots/spend`, { level });
}

export function restoreSpellSlots(campaignId: string, charId: string, level?: number): Promise<Character> {
  return apiPost<Character>(`/campaigns/${campaignId}/characters/${charId}/spell-slots/restore`, { level });
}

export function generateVillainProfile(campaignId: string, charId: string): Promise<{ villain_profile: string }> {
  return apiPost<{ villain_profile: string }>(`/campaigns/${campaignId}/characters/${charId}/villain-profile`);
}

export async function uploadPortrait(campaignId: string, charId: string, file: File): Promise<Character> {
  const form = new FormData();
  form.append('file', file);
  const res = await fetch(`${BASE_URL}/campaigns/${campaignId}/characters/${charId}/portrait`, {
    method: 'POST',
    body: form,
  });
  if (!res.ok) throw new Error(`Upload failed: ${res.status}`);
  return res.json() as Promise<Character>;
}
