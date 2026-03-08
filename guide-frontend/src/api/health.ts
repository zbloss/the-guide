import { apiGet } from './client';
import type { HealthResponse, VersionResponse } from './types';

export function getHealth(): Promise<HealthResponse> {
  return apiGet<HealthResponse>('/health');
}

export function getVersion(): Promise<VersionResponse> {
  return apiGet<VersionResponse>('/version');
}
