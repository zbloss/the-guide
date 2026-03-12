import { apiGet, apiPost, apiPut, apiDelete } from './client';
import type { Campaign, ChatMessage, CreateCampaignRequest, UpdateCampaignRequest } from './types';

export function listCampaigns(): Promise<Campaign[]> {
  return apiGet<Campaign[]>('/campaigns');
}

export function createCampaign(data: CreateCampaignRequest): Promise<Campaign> {
  return apiPost<Campaign>('/campaigns', data);
}

export function getCampaign(id: string): Promise<Campaign> {
  return apiGet<Campaign>(`/campaigns/${id}`);
}

export function updateCampaign(id: string, data: UpdateCampaignRequest): Promise<Campaign> {
  return apiPut<Campaign>(`/campaigns/${id}`, data);
}

export function deleteCampaign(id: string): Promise<void> {
  return apiDelete(`/campaigns/${id}`);
}

export function getChatHistory(id: string, limit = 50): Promise<ChatMessage[]> {
  return apiGet<ChatMessage[]>(`/campaigns/${id}/chat/history?limit=${limit}`);
}
