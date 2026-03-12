import { apiGet, apiPost, apiDelete } from './client';
import type { HomebrewRule, CreateHomebrewRuleRequest } from './types';

export function listHomebrewRules(campaignId: string): Promise<HomebrewRule[]> {
  return apiGet<HomebrewRule[]>(`/campaigns/${campaignId}/homebrew`);
}

export function createHomebrewRule(campaignId: string, data: CreateHomebrewRuleRequest): Promise<HomebrewRule> {
  return apiPost<HomebrewRule>(`/campaigns/${campaignId}/homebrew`, data);
}

export function deleteHomebrewRule(campaignId: string, id: string): Promise<void> {
  return apiDelete(`/campaigns/${campaignId}/homebrew/${id}`);
}
