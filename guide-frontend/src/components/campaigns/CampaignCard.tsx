import { Link } from 'react-router-dom';
import { Badge } from '../common/Badge';
import { ConfirmButton } from '../common/ConfirmButton';
import type { Campaign } from '../../api/types';

interface CampaignCardProps {
  campaign: Campaign;
  onDelete: (id: string) => void;
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
      <div className="card-actions">
        <Link to={`/campaigns/${campaign.id}`} className="btn btn-sm">Open</Link>
        <ConfirmButton
          label="Delete"
          variant="danger"
          onConfirm={() => onDelete(campaign.id)}
        />
      </div>
    </div>
  );
}
