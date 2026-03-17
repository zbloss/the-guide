import { apiGet, apiPost, apiDelete, BASE_URL } from './client';
import type { Session, SessionEvent, SessionSummary, CreateSessionRequest, CreateSessionEventRequest, Perspective, ImprovPromptResponse, TranscribeResponse } from './types';

export async function uploadSessionMap(campaignId: string, sessionId: string, file: File): Promise<Session> {
  const form = new FormData();
  form.append('file', file);
  const res = await fetch(`${BASE_URL}/campaigns/${campaignId}/sessions/${sessionId}/map`, {
    method: 'POST',
    body: form,
  });
  if (!res.ok) throw new Error(`Upload failed: ${res.status}`);
  return res.json();
}

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

export function deleteEvent(campaignId: string, sessionId: string, eventId: string): Promise<void> {
  return apiDelete(`/campaigns/${campaignId}/sessions/${sessionId}/events/${eventId}`);
}

export function getImprovPrompt(campaignId: string, sessionId: string): Promise<ImprovPromptResponse> {
  return apiPost<ImprovPromptResponse>(`/campaigns/${campaignId}/sessions/${sessionId}/improv-prompt`);
}

export async function exportSessionSummary(campaignId: string, sessionId: string, perspective: string = 'dm'): Promise<void> {
  const response = await fetch(`${BASE_URL}/campaigns/${campaignId}/sessions/${sessionId}/summary/export?perspective=${perspective}`, { method: 'GET' });
  if (!response.ok) throw new Error('Export failed');
  const blob = await response.blob();
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `session-summary.md`;
  a.click();
  URL.revokeObjectURL(url);
}

export function generateDebrief(campaignId: string, sessionId: string): Promise<{ debrief: string }> {
  return apiPost<{ debrief: string }>(`/campaigns/${campaignId}/sessions/${sessionId}/debrief`);
}

export function generateSessionPrep(campaignId: string, upcomingNotes?: string): Promise<{ prep: string }> {
  return apiPost<{ prep: string }>(`/campaigns/${campaignId}/sessions/prep`, { upcoming_notes: upcomingNotes });
}

export async function transcribeAudio(campaignId: string, sessionId: string, audioBlob: Blob): Promise<TranscribeResponse> {
  const form = new FormData();
  form.append('audio', audioBlob, 'recording.webm');
  const res = await fetch(`${BASE_URL}/campaigns/${campaignId}/sessions/${sessionId}/transcribe`, {
    method: 'POST',
    body: form,
  });
  if (!res.ok) throw new Error(`Transcription failed: ${res.status}`);
  return res.json() as Promise<TranscribeResponse>;
}
