import { apiGet, apiPost, apiMultipart, apiDelete } from './client';
import type { CampaignDocument, DocumentPageOcr, GlobalDocument, RankedChunk } from './types';

// Campaign documents
export function listCampaignDocs(campaignId: string): Promise<CampaignDocument[]> {
  return apiGet<CampaignDocument[]>(`/campaigns/${campaignId}/documents`);
}

export function uploadCampaignDoc(campaignId: string, fileOrForm: File | FormData): Promise<CampaignDocument> {
  const fd = fileOrForm instanceof FormData ? fileOrForm : (() => { const f = new FormData(); f.append('file', fileOrForm); return f; })();
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

export function deleteCampaignDoc(campaignId: string, docId: string): Promise<void> {
  return apiDelete(`/campaigns/${campaignId}/documents/${docId}`);
}

export function deleteGlobalDoc(docId: string): Promise<void> {
  return apiDelete(`/documents/${docId}`);
}

export function getCampaignDocPages(campaignId: string, docId: string): Promise<DocumentPageOcr[]> {
  return apiGet<DocumentPageOcr[]>(`/campaigns/${campaignId}/documents/${docId}/pages`);
}

// Search
export function searchRules(q: string): Promise<RankedChunk[]> {
  return apiGet<RankedChunk[]>(`/rules/search?q=${encodeURIComponent(q)}`);
}

export function searchCampaignDoc(campaignId: string, docId: string, q: string): Promise<RankedChunk[]> {
  return apiGet<RankedChunk[]>(`/campaigns/${campaignId}/documents/${docId}/search?q=${encodeURIComponent(q)}`);
}
