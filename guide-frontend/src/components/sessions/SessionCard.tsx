import { Link } from 'react-router-dom';
import { Badge } from '../common/Badge';
import { ConfirmButton } from '../common/ConfirmButton';
import type { Session } from '../../api/types';
import { deriveSessionStatus } from '../../api/types';

interface SessionCardProps {
  session: Session;
  campaignId: string;
  onDelete: (id: string) => void;
}

export function SessionCard({ session, campaignId, onDelete }: SessionCardProps) {
  const status = deriveSessionStatus(session);
  const statusVariant = status === 'ended' ? 'default' : status === 'started' ? 'success' : 'info';

  return (
    <div className="card session-card">
      <div className="card-header">
        <Link to={`/campaigns/${campaignId}/sessions/${session.id}`} className="card-title">
          Session {session.session_number}{session.title ? `: ${session.title}` : ''}
        </Link>
        <Badge label={status} variant={statusVariant} />
      </div>
      <div className="card-meta">
        {session.started_at && <span>Started: {new Date(session.started_at).toLocaleDateString()}</span>}
        {session.ended_at && <span>Ended: {new Date(session.ended_at).toLocaleDateString()}</span>}
      </div>
      <div className="card-actions">
        <Link to={`/campaigns/${campaignId}/sessions/${session.id}`} className="btn btn-sm">Open</Link>
        <ConfirmButton label="Delete" variant="danger" onConfirm={() => onDelete(session.id)} />
      </div>
    </div>
  );
}
