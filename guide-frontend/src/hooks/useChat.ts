import { useState, useCallback, useRef, useEffect } from 'react';
import { startChatStream } from '../api/chat';
import { getChatHistory } from '../api/campaigns';

export interface ChatMessage {
  role: 'user' | 'assistant';
  content: string;
  streaming?: boolean;
}

function historyKey(campaignId: string) {
  return `chat_history_${campaignId}`;
}

function loadHistory(campaignId: string): ChatMessage[] {
  try {
    const raw = localStorage.getItem(historyKey(campaignId));
    if (!raw) return [];
    return (JSON.parse(raw) as ChatMessage[]).filter((m) => !m.streaming);
  } catch {
    return [];
  }
}

function saveHistory(campaignId: string, messages: ChatMessage[]) {
  const toSave = messages.filter((m) => !m.streaming);
  localStorage.setItem(historyKey(campaignId), JSON.stringify(toSave));
}

export function useChat(campaignId: string) {
  const [messages, setMessages] = useState<ChatMessage[]>(() => loadHistory(campaignId));
  const [streaming, setStreaming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const abortRef = useRef<AbortController | null>(null);

  // On mount, fetch persistent history from the backend and replace local cache
  useEffect(() => {
    let cancelled = false;
    getChatHistory(campaignId)
      .then((history) => {
        if (cancelled) return;
        if (history.length > 0) {
          const mapped: ChatMessage[] = history.map((m) => ({
            role: m.role,
            content: m.content,
          }));
          setMessages(mapped);
          saveHistory(campaignId, mapped);
        }
      })
      .catch(() => {
        // Backend history fetch failed — keep localStorage fallback already in state
      });
    return () => { cancelled = true; };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [campaignId]);

  // Persist to localStorage whenever messages change (non-streaming)
  useEffect(() => {
    if (!streaming) {
      saveHistory(campaignId, messages);
    }
  }, [messages, streaming, campaignId]);

  const sendMessage = useCallback(async (message: string) => {
    if (streaming) return;

    setError(null);
    setMessages((prev) => [...prev, { role: 'user', content: message }]);
    setStreaming(true);

    // Append a streaming placeholder for the assistant
    setMessages((prev) => [...prev, { role: 'assistant', content: '', streaming: true }]);

    abortRef.current = new AbortController();

    try {
      const response = await startChatStream(campaignId, message, abortRef.current.signal);

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }

      const reader = response.body!.getReader();
      const decoder = new TextDecoder();
      let buffer = '';

      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const parts = buffer.split('\n\n');
        buffer = parts.pop() ?? '';

        for (const part of parts) {
          const eventLine = part.split('\n').find((l) => l.startsWith('event:'));
          const dataLine = part.split('\n').find((l) => l.startsWith('data:'));
          const event = eventLine?.slice(7).trim();
          const data = dataLine?.slice(5).replace(/^ /, '');

          if (event === 'token' && data) {
            setMessages((prev) => {
              const updated = [...prev];
              const last = updated[updated.length - 1];
              if (last?.role === 'assistant') {
                updated[updated.length - 1] = { ...last, content: last.content + data };
              }
              return updated;
            });
          } else if (event === 'done') {
            setMessages((prev) => {
              const updated = [...prev];
              const last = updated[updated.length - 1];
              if (last?.role === 'assistant') {
                updated[updated.length - 1] = { ...last, streaming: false };
              }
              return updated;
            });
          } else if (event === 'error' && data) {
            setError(data);
            setMessages((prev) => {
              const updated = [...prev];
              const last = updated[updated.length - 1];
              if (last?.role === 'assistant' && last.streaming) {
                updated.pop();
              }
              return updated;
            });
          }
        }
      }
    } catch (err: unknown) {
      if (err instanceof Error && err.name === 'AbortError') {
        // User cancelled — remove streaming placeholder
        setMessages((prev) => {
          const updated = [...prev];
          const last = updated[updated.length - 1];
          if (last?.role === 'assistant' && last.streaming) {
            updated.pop();
          }
          return updated;
        });
      } else {
        setError(err instanceof Error ? err.message : String(err));
        setMessages((prev) => {
          const updated = [...prev];
          const last = updated[updated.length - 1];
          if (last?.role === 'assistant' && last.streaming) {
            updated.pop();
          }
          return updated;
        });
      }
    } finally {
      setStreaming(false);
      abortRef.current = null;
    }
  }, [campaignId, streaming]);

  const cancel = useCallback(() => {
    abortRef.current?.abort();
  }, []);

  const clearMessages = useCallback(() => {
    setMessages([]);
    setError(null);
    localStorage.removeItem(historyKey(campaignId));
  }, [campaignId]);

  return { messages, streaming, error, sendMessage, cancel, clearMessages };
}
