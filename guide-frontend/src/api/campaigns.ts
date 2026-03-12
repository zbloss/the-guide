import { apiGet, apiPost, apiPut, apiDelete } from './client';
import type { Campaign, CampaignAnalytics, ChatMessage, ConsistencyReport, CreateCampaignRequest, SearchResults, UpdateCampaignRequest } from './types';

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

export function generatePlotTwist(campaignId: string, tone?: string): Promise<{ twist: string; tone: string }> {
  return apiPost(`/campaigns/${campaignId}/plot-twist`, { tone: tone ?? 'dramatic' });
}

export function runConsistencyCheck(campaignId: string): Promise<ConsistencyReport> {
  return apiPost<ConsistencyReport>(`/campaigns/${campaignId}/consistency-check`);
}

export function searchCampaign(campaignId: string, query: string): Promise<SearchResults> {
  return apiGet<SearchResults>(`/campaigns/${campaignId}/search?q=${encodeURIComponent(query)}`);
}

export function getCampaignAnalytics(campaignId: string): Promise<CampaignAnalytics> {
  return apiGet<CampaignAnalytics>(`/campaigns/${campaignId}/analytics`);
}

export function generateShareToken(campaignId: string): Promise<Campaign> {
  return apiPost<Campaign>(`/campaigns/${campaignId}/share`);
}

export function getSharedView(token: string): Promise<unknown> {
  return apiGet(`/view/${token}`);
}
