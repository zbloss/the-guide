import { ParticipantRow } from './ParticipantRow';
import type { EncounterSummary } from '../../api/types';

interface CombatTrackerProps {
  encounter: EncounterSummary;
  campaignId: string;
  onUpdate: (updated: EncounterSummary) => void;
  readOnly?: boolean;
}

export function CombatTracker({ encounter, campaignId, onUpdate, readOnly }: CombatTrackerProps) {
  const sorted = [...encounter.participants].sort((a, b) =>
    b.initiative_total - a.initiative_total || a.id.localeCompare(b.id)
  );
  const currentParticipant = sorted.length > 0 ? sorted[encounter.current_turn_index % sorted.length] : undefined;

  return (
    <div className="combat-tracker">
      <div className="combat-header">
        <span className="round-counter">Round {encounter.round}</span>
        {currentParticipant && (
          <span className="current-turn-name">Current Turn: <strong>{currentParticipant.name}</strong></span>
        )}
        {encounter.status === 'active' && !readOnly && (
          <span className="keyboard-hint">Press <kbd>Space</kbd> for next turn</span>
        )}
      </div>

      <div className="combat-table-wrapper">
        <table className="data-table combat-table">
          <thead>
            <tr>
              <th>Name</th>
              <th>Init</th>
              <th>HP</th>
              <th>AC</th>
              <th>Conditions</th>
              <th>Budget</th>
              {!readOnly && <th>Controls</th>}
            </tr>
          </thead>
          <tbody>
            {sorted.map((p, idx) => (
              <ParticipantRow
                key={p.id}
                participant={p}
                isCurrentTurn={sorted.length > 0 && idx === encounter.current_turn_index % sorted.length}
                campaignId={campaignId}
                encounterId={encounter.id}
                onUpdate={onUpdate}
                readOnly={readOnly}
              />
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
