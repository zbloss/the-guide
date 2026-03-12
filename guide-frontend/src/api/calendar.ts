import { apiGet, apiPost, apiDelete } from './client';
import type { CalendarEntry, CreateCalendarEntryRequest } from './types';

export function listCalendarEntries(campaignId: string): Promise<CalendarEntry[]> {
  return apiGet<CalendarEntry[]>(`/campaigns/${campaignId}/calendar`);
}

export function createCalendarEntry(
  campaignId: string,
  data: CreateCalendarEntryRequest,
): Promise<CalendarEntry> {
  return apiPost<CalendarEntry>(`/campaigns/${campaignId}/calendar`, data);
}

export function deleteCalendarEntry(campaignId: string, entryId: string): Promise<void> {
  return apiDelete(`/campaigns/${campaignId}/calendar/${entryId}`);
}
