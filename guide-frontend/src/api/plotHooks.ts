import { apiGet, apiPost, apiPut, apiDelete } from './client';
import type { TrackedPlotHook, CreateTrackedPlotHookRequest, UpdateTrackedPlotHookRequest } from './types';

export function listPlotHooks(campaignId: string, charId: string): Promise<TrackedPlotHook[]> {
  return apiGet<TrackedPlotHook[]>(`/campaigns/${campaignId}/characters/${charId}/plot-hooks`);
}

export function createPlotHook(
  campaignId: string,
  charId: string,
  req: CreateTrackedPlotHookRequest,
): Promise<TrackedPlotHook> {
  return apiPost<TrackedPlotHook>(`/campaigns/${campaignId}/characters/${charId}/plot-hooks`, req);
}

export function updatePlotHook(
  campaignId: string,
  charId: string,
  hookId: string,
  req: UpdateTrackedPlotHookRequest,
): Promise<TrackedPlotHook> {
  return apiPut<TrackedPlotHook>(
    `/campaigns/${campaignId}/characters/${charId}/plot-hooks/${hookId}`,
    req,
  );
}

export function deletePlotHook(campaignId: string, charId: string, hookId: string): Promise<void> {
  return apiDelete(`/campaigns/${campaignId}/characters/${charId}/plot-hooks/${hookId}`);
}
