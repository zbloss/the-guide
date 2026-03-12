import { apiGet, apiPost, apiDelete } from './client';
import type { EncounterTemplate } from './types';

export function listTemplates(): Promise<EncounterTemplate[]> {
  return apiGet<EncounterTemplate[]>('/encounter-templates');
}

export function getTemplate(id: string): Promise<EncounterTemplate> {
  return apiGet<EncounterTemplate>(`/encounter-templates/${id}`);
}

export function saveTemplate(
  campaignId: string,
  encounterId: string,
  name?: string,
): Promise<EncounterTemplate> {
  return apiPost<EncounterTemplate>(
    `/campaigns/${campaignId}/encounters/${encounterId}/save-as-template`,
    { name },
  );
}

export function deleteTemplate(id: string): Promise<void> {
  return apiDelete(`/encounter-templates/${id}`);
}
