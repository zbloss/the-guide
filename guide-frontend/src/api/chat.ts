import { BASE_URL } from './client';
import type { Perspective } from './types';

// SSE streaming chat — uses fetch + ReadableStream, NOT EventSource
// The chat endpoint is POST so EventSource cannot be used
export function startChatStream(
  campaignId: string,
  message: string,
  perspective: Perspective,
  signal: AbortSignal,
): Promise<Response> {
  return fetch(`${BASE_URL}/campaigns/${campaignId}/chat`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ message, perspective }),
    signal,
  });
}
