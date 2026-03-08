import { apiGet, apiPost, apiMultipart } from './client';
import type { CampaignDocument, GlobalDocument } from './types';

// Campaign documents
export function listCampaignDocs(campaignId: string): Promise<CampaignDocument[]> {
  return apiGet<CampaignDocument[]>(`/campaigns/${campaignId}/documents`);
}

export function uploadCampaignDoc(campaignId: string, file: File): Promise<CampaignDocument> {
  const fd = new FormData();
  fd.append('file', file);
  return apiMultipart<CampaignDocument>(`/campaigns/${campaignId}/documents`, fd);
}

export function getCampaignDoc(campaignId: string, docId: string): Promise<CampaignDocument> {
  return apiGet<CampaignDocument>(`/campaigns/${campaignId}/documents/${docId}`);
}

export function ingestCampaignDoc(campaignId: string, docId: string): Promise<CampaignDocument> {
  return apiPost<CampaignDocument>(`/campaigns/${campaignId}/documents/${docId}/ingest`);
}

// Global documents
export function listGlobalDocs(): Promise<GlobalDocument[]> {
  return apiGet<GlobalDocument[]>('/documents');
}

export function uploadGlobalDoc(file: File): Promise<GlobalDocument> {
  const fd = new FormData();
  fd.append('file', file);
  return apiMultipart<GlobalDocument>('/documents', fd);
}

export function getGlobalDoc(docId: string): Promise<GlobalDocument> {
  return apiGet<GlobalDocument>(`/documents/${docId}`);
}

export function ingestGlobalDoc(docId: string): Promise<GlobalDocument> {
  return apiPost<GlobalDocument>(`/documents/${docId}/ingest`);
}
