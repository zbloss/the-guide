import { SessionCard } from './SessionCard';
import type { Session } from '../../api/types';

interface SessionListProps {
  sessions: Session[];
  campaignId: string;
  onDelete: (id: string) => void;
}

export function SessionList({ sessions, campaignId, onDelete }: SessionListProps) {
  if (sessions.length === 0) {
    return <p className="empty-state">No sessions yet.</p>;
  }
  return (
    <div className="card-list">
      {sessions.map((s) => (
        <SessionCard key={s.id} session={s} campaignId={campaignId} onDelete={onDelete} />
      ))}
    </div>
  );
}
