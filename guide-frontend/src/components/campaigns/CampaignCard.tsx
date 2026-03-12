import { Link } from 'react-router-dom';
import { Badge } from '../common/Badge';
import { ConfirmButton } from '../common/ConfirmButton';
import type { Campaign } from '../../api/types';

interface CampaignCardProps {
  campaign: Campaign;
  onDelete: (id: string) => void;
}

function relativeTime(dateStr: string): string {
  const diff = Date.now() - new Date(dateStr).getTime();
  const days = Math.floor(diff / 86400000);
  if (days === 0) return 'Today';
  if (days === 1) return 'Yesterday';
  if (days < 7) return `${days} days ago`;
  const weeks = Math.floor(days / 7);
  if (weeks < 4) return `${weeks}w ago`;
  return new Date(dateStr).toLocaleDateString();
}

export function CampaignCard({ campaign, onDelete }: CampaignCardProps) {
  return (
    <div className="card campaign-card">
      <div className="card-header">
        <Link to={`/campaigns/${campaign.id}`} className="card-title">
          {campaign.name}
        </Link>
        <Badge label={campaign.game_system} variant="info" />
      </div>
      {campaign.description && (
        <p className="card-description">{campaign.description}</p>
      )}
      <div className="card-meta">
        <span>Updated {relativeTime(campaign.updated_at)}</span>
      </div>
      <div className="card-actions">
        <Link to={`/campaigns/${campaign.id}`} className="btn btn-sm btn-primary">Open Campaign</Link>
        <Link to={`/campaigns/${campaign.id}/characters`} className="btn btn-sm">Add Character</Link>
        <ConfirmButton
          label="Delete"
          variant="danger"
          onConfirm={() => onDelete(campaign.id)}
        />
      </div>
    </div>
  );
}
