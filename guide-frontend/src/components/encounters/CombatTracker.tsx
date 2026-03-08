import { ParticipantRow } from './ParticipantRow';
import type { EncounterSummary } from '../../api/types';

interface CombatTrackerProps {
  encounter: EncounterSummary;
  campaignId: string;
  onUpdate: (updated: EncounterSummary) => void;
}

export function CombatTracker({ encounter, campaignId, onUpdate }: CombatTrackerProps) {
  const sorted = [...encounter.participants].sort((a, b) => b.initiative_total - a.initiative_total);
  const currentParticipant = sorted[encounter.current_turn_index % sorted.length];

  return (
    <div className="combat-tracker">
      <div className="combat-header">
        <span className="round-counter">Round {encounter.current_round}</span>
        {currentParticipant && (
          <span className="current-turn-name">Current Turn: <strong>{currentParticipant.name}</strong></span>
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
              <th>Controls</th>
            </tr>
          </thead>
          <tbody>
            {sorted.map((p, idx) => (
              <ParticipantRow
                key={p.id}
                participant={p}
                isCurrentTurn={idx === encounter.current_turn_index % sorted.length}
                campaignId={campaignId}
                encounterId={encounter.id}
                onUpdate={onUpdate}
              />
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
