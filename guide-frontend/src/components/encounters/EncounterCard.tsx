import { Link } from 'react-router-dom';
import { Badge } from '../common/Badge';
import { ConfirmButton } from '../common/ConfirmButton';
import type { EncounterSummary } from '../../api/types';

interface EncounterCardProps {
  encounter: EncounterSummary;
  campaignId: string;
  onDelete: (id: string) => void;
}

export function EncounterCard({ encounter, campaignId, onDelete }: EncounterCardProps) {
  const statusVariant =
    encounter.status === 'active' ? 'warning' :
    encounter.status === 'completed' ? 'success' : 'default';

  return (
    <div className="card encounter-card">
      <div className="card-header">
        <Link to={`/campaigns/${campaignId}/encounters/${encounter.id}`} className="card-title">
          {encounter.name}
        </Link>
        <Badge label={encounter.status} variant={statusVariant} />
      </div>
      {encounter.description && <p className="card-description">{encounter.description}</p>}
      <div className="card-meta">
        <span>{encounter.participants.length} participants</span>
        {encounter.status === 'active' && <span>Round {encounter.current_round}</span>}
      </div>
      <div className="card-actions">
        <Link to={`/campaigns/${campaignId}/encounters/${encounter.id}`} className="btn btn-sm">Open</Link>
        <ConfirmButton label="Delete" variant="danger" onConfirm={() => onDelete(encounter.id)} />
      </div>
    </div>
  );
}
