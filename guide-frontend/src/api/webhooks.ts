import { apiGet, apiPost, apiDelete } from './client';
import type { CampaignWebhook, CreateWebhookRequest } from './types';

export function listWebhooks(campaignId: string): Promise<CampaignWebhook[]> {
  return apiGet<CampaignWebhook[]>(`/campaigns/${campaignId}/webhooks`);
}

export function createWebhook(
  campaignId: string,
  req: CreateWebhookRequest,
): Promise<CampaignWebhook> {
  return apiPost<CampaignWebhook>(`/campaigns/${campaignId}/webhooks`, req);
}

export function deleteWebhook(campaignId: string, webhookId: string): Promise<void> {
  return apiDelete(`/campaigns/${campaignId}/webhooks/${webhookId}`);
}
