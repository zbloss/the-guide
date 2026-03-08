import { apiGet, apiPost, apiDelete } from './client';
import type { Session, SessionEvent, SessionSummary, CreateSessionRequest, CreateSessionEventRequest, Perspective } from './types';

export function listSessions(campaignId: string): Promise<Session[]> {
  return apiGet<Session[]>(`/campaigns/${campaignId}/sessions`);
}

export function createSession(campaignId: string, data: CreateSessionRequest): Promise<Session> {
  return apiPost<Session>(`/campaigns/${campaignId}/sessions`, data);
}

export function getSession(campaignId: string, sessionId: string): Promise<Session> {
  return apiGet<Session>(`/campaigns/${campaignId}/sessions/${sessionId}`);
}

export function deleteSession(campaignId: string, sessionId: string): Promise<void> {
  return apiDelete(`/campaigns/${campaignId}/sessions/${sessionId}`);
}

export function startSession(campaignId: string, sessionId: string): Promise<Session> {
  return apiPost<Session>(`/campaigns/${campaignId}/sessions/${sessionId}/start`);
}

export function endSession(campaignId: string, sessionId: string): Promise<Session> {
  return apiPost<Session>(`/campaigns/${campaignId}/sessions/${sessionId}/end`);
}

export function listEvents(campaignId: string, sessionId: string): Promise<SessionEvent[]> {
  return apiGet<SessionEvent[]>(`/campaigns/${campaignId}/sessions/${sessionId}/events`);
}

export function createEvent(campaignId: string, sessionId: string, data: CreateSessionEventRequest): Promise<SessionEvent> {
  return apiPost<SessionEvent>(`/campaigns/${campaignId}/sessions/${sessionId}/events`, data);
}

export function getSessionSummary(campaignId: string, sessionId: string, perspective: Perspective): Promise<SessionSummary> {
  return apiGet<SessionSummary>(`/campaigns/${campaignId}/sessions/${sessionId}/summary?perspective=${perspective}`);
}
