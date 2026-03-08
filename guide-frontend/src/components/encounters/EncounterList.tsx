import { EncounterCard } from './EncounterCard';
import type { EncounterSummary } from '../../api/types';

interface EncounterListProps {
  encounters: EncounterSummary[];
  campaignId: string;
  onDelete: (id: string) => void;
}

export function EncounterList({ encounters, campaignId, onDelete }: EncounterListProps) {
  if (encounters.length === 0) {
    return <p className="empty-state">No encounters yet.</p>;
  }
  return (
    <div className="card-list">
      {encounters.map((e) => (
        <EncounterCard key={e.id} encounter={e} campaignId={campaignId} onDelete={onDelete} />
      ))}
    </div>
  );
}
