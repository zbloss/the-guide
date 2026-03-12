import { apiGet, apiPost, apiDelete } from './client';
import type { CreateLootItemRequest, LootItem } from './types';

export function listLoot(campaignId: string, sessionId: string): Promise<LootItem[]> {
  return apiGet<LootItem[]>(`/campaigns/${campaignId}/sessions/${sessionId}/loot`);
}

export function createLootItem(
  campaignId: string,
  sessionId: string,
  data: CreateLootItemRequest,
): Promise<LootItem> {
  return apiPost<LootItem>(`/campaigns/${campaignId}/sessions/${sessionId}/loot`, data);
}

export function deleteLootItem(
  campaignId: string,
  sessionId: string,
  itemId: string,
): Promise<void> {
  return apiDelete(`/campaigns/${campaignId}/sessions/${sessionId}/loot/${itemId}`);
}
