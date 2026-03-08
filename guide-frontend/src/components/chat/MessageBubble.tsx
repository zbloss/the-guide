import Markdown from 'react-markdown';
import type { ChatMessage } from '../../hooks/useChat';

interface MessageBubbleProps {
  message: ChatMessage;
}

export function MessageBubble({ message }: MessageBubbleProps) {
  const isUser = message.role === 'user';
  return (
    <div className={`message-bubble ${isUser ? 'message-user' : 'message-assistant'}`}>
      <div className="message-content">
        {message.streaming && !message.content
          ? <span className="streaming-dots">…</span>
          : <Markdown>{message.content}</Markdown>
        }
      </div>
    </div>
  );
}
