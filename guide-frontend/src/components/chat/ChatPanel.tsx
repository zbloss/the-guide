import { useState, useRef, useEffect } from 'react';
import { MessageBubble } from './MessageBubble';
import { ErrorBanner } from '../common/ErrorBanner';
import { useChat } from '../../hooks/useChat';

interface ChatPanelProps {
  campaignId: string;
}

export function ChatPanel({ campaignId }: ChatPanelProps) {
  const [input, setInput] = useState('');
  const { messages, streaming, error, sendMessage, cancel, clearMessages } = useChat(campaignId);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const handleSend = () => {
    const text = input.trim();
    if (!text || streaming) return;
    setInput('');
    sendMessage(text);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div className="chat-panel">
      <div className="chat-toolbar">
        <button className="btn btn-sm" onClick={clearMessages} disabled={streaming}>Clear</button>
      </div>

      {error && <ErrorBanner message={error} />}

      <div className="chat-messages">
        {messages.length === 0 && (
          <p className="empty-state">Ask anything about your campaign…</p>
        )}
        {messages.map((msg, i) => (
          <MessageBubble key={i} message={msg} />
        ))}
        <div ref={bottomRef} />
      </div>

      <div className="chat-input-row">
        <textarea
          className="chat-input"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Ask the Guide…"
          rows={2}
          disabled={streaming}
        />
        <button
          className="btn btn-primary chat-send"
          onClick={handleSend}
          disabled={streaming || !input.trim()}
        >
          {streaming ? '…' : 'Send'}
        </button>
        {streaming && <button className="btn btn-sm" onClick={cancel}>Cancel</button>}
      </div>
    </div>
  );
}
