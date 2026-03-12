import { useState } from 'react';
import { spendSpellSlot, restoreSpellSlots } from '../../api/characters';
import type { Character } from '../../api/types';

interface SpellSlotPanelProps {
  campaignId: string;
  character: Character;
  onUpdate: (c: Character) => void;
}

export function SpellSlotPanel({ campaignId, character, onUpdate }: SpellSlotPanelProps) {
  const [error, setError] = useState('');

  if (!character.spell_slots || character.spell_slots.length === 0) {
    return null;
  }

  const handleSpend = async (level: number) => {
    try {
      setError('');
      const updated = await spendSpellSlot(campaignId, character.id, level);
      onUpdate(updated);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const handleRestoreAll = async () => {
    try {
      setError('');
      const updated = await restoreSpellSlots(campaignId, character.id);
      onUpdate(updated);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div className="spell-slot-panel">
      <div className="section-header">
        <h3>Spell Slots</h3>
        <button className="btn btn-sm" onClick={handleRestoreAll}>Restore All (Long Rest)</button>
      </div>
      {error && <div className="form-error-banner">{error}</div>}
      <div className="spell-slot-grid">
        {character.spell_slots.map((slot) => (
          <div key={slot.level} className="spell-slot-row">
            <span className="spell-slot-label">Level {slot.level}</span>
            <div className="spell-slot-pips">
              {Array.from({ length: slot.total }, (_, i) => (
                <button
                  key={i}
                  className={`spell-pip ${i < slot.remaining ? 'pip-full' : 'pip-empty'}`}
                  onClick={() => { if (i < slot.remaining) void handleSpend(slot.level); }}
                  title={i < slot.remaining ? `Spend level ${slot.level} slot` : 'Spent'}
                />
              ))}
            </div>
            <span className="spell-slot-count">{slot.remaining}/{slot.total}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
