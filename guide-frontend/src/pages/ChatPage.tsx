import { useParams } from 'react-router-dom';
import { ChatPanel } from '../components/chat/ChatPanel';

export function ChatPage() {
  const { campaignId } = useParams<{ campaignId: string }>();

  return (
    <div className="page chat-page">
      <div className="page-header">
        <h1>Campaign Chat</h1>
      </div>
      <ChatPanel campaignId={campaignId!} />
    </div>
  );
}
