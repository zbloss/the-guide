import { useState } from 'react';
import { ConditionBadge } from '../characters/ConditionBadge';
import { updateParticipant } from '../../api/encounters';
import type { CombatParticipant, Condition, EncounterSummary } from '../../api/types';
import { ALL_CONDITIONS } from '../../api/types';

interface ParticipantRowProps {
  participant: CombatParticipant;
  isCurrentTurn: boolean;
  campaignId: string;
  encounterId: string;
  onUpdate: (updated: EncounterSummary) => void;
}

export function ParticipantRow({ participant: p, isCurrentTurn, campaignId, encounterId, onUpdate }: ParticipantRowProps) {
  const [damage, setDamage] = useState('');
  const [heal, setHeal] = useState('');
  const [setHpVal, setSetHpVal] = useState('');
  const [addCond, setAddCond] = useState<Condition>('blinded');
  const [loading, setLoading] = useState(false);
  const [rowError, setRowError] = useState('');
  const [editingName, setEditingName] = useState(false);
  const [nameInput, setNameInput] = useState(p.name);

  const pct = p.max_hp > 0 ? (p.current_hp / p.max_hp) * 100 : 0;
  const hpOverflow = p.current_hp > p.max_hp;
  const hpClass = hpOverflow ? 'hp-overflow' : pct > 50 ? 'hp-high' : pct > 25 ? 'hp-mid' : 'hp-low';
  const hpLabel = hpOverflow ? `${p.current_hp}/${p.max_hp} (+${p.current_hp - p.max_hp})` : `${p.current_hp}/${p.max_hp}`;

  const doUpdate = async (req: Parameters<typeof updateParticipant>[3]) => {
    setRowError('');
    setLoading(true);
    try {
      const updated = await updateParticipant(campaignId, encounterId, p.id, req);
      onUpdate(updated);
    } catch (e) {
      setRowError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  const handleSaveName = async () => {
    if (!nameInput.trim() || nameInput === p.name) { setEditingName(false); return; }
    await doUpdate({ name: nameInput.trim() });
    setEditingName(false);
  };

  const availableToAdd = ALL_CONDITIONS.filter((c) => !p.conditions.includes(c));

  return (
    <>
    <tr className={`participant-row ${isCurrentTurn ? 'current-turn' : ''} ${loading ? 'row-loading' : ''}`}>
      <td className="participant-name">
        {isCurrentTurn && <span className="turn-indicator">▶ </span>}
        {editingName ? (
          <span className="inline-name-edit">
            <input
              className="control-input"
              style={{ width: 100 }}
              value={nameInput}
              onChange={(e) => setNameInput(e.target.value)}
              onKeyDown={(e) => { if (e.key === 'Enter') handleSaveName(); if (e.key === 'Escape') setEditingName(false); }}
              autoFocus
            />
            <button className="btn btn-sm btn-primary" onClick={handleSaveName}>✓</button>
            <button className="btn btn-sm" onClick={() => setEditingName(false)}>✕</button>
          </span>
        ) : (
          <span>
            <strong>{p.name}</strong>
            <button
              className="btn-icon-inline"
              title="Edit name"
              onClick={() => { setNameInput(p.name); setEditingName(true); }}
            >✏️</button>
          </span>
        )}
      </td>
      <td className="participant-init">{p.initiative_total}</td>
      <td className="participant-hp">
        <div className="hp-bar-container">
          <div className={`hp-bar-fill ${hpClass}`} style={{ width: `${Math.min(100, pct)}%` }} />
          <span className="hp-bar-label">{hpLabel}</span>
        </div>
      </td>
      <td className="participant-ac">{p.armor_class}</td>
      <td className="participant-conditions">
        {p.conditions.map((c) => (
          <ConditionBadge
            key={c}
            condition={c}
            onRemove={() => doUpdate({ remove_condition: c })}
          />
        ))}
      </td>
      <td className="participant-budget">
        <span className={`budget-icon ${p.action_budget.has_action ? '' : 'spent'}`} title="Action">A</span>
        <span className={`budget-icon ${p.action_budget.has_bonus_action ? '' : 'spent'}`} title="Bonus">B</span>
        <span className={`budget-icon ${p.action_budget.has_reaction ? '' : 'spent'}`} title="Reaction">R</span>
        <span className={`budget-icon ${p.action_budget.movement_remaining > 0 ? '' : 'spent'}`} title={`Move: ${p.action_budget.movement_remaining}ft`}>M</span>
      </td>
      <td className="participant-controls">
        {/* Damage */}
        <div className="control-row">
          <input className="control-input" type="number" min={0} placeholder="DMG" value={damage} onChange={(e) => setDamage(e.target.value)} />
          <button className="btn btn-sm btn-danger" onClick={() => { doUpdate({ hp_delta: -Math.abs(Number(damage)) }); setDamage(''); }} disabled={!damage || loading}>Hit</button>
        </div>
        {/* Heal */}
        <div className="control-row">
          <input className="control-input" type="number" min={0} placeholder="Heal" value={heal} onChange={(e) => setHeal(e.target.value)} />
          <button className="btn btn-sm btn-success" onClick={() => { doUpdate({ hp_delta: Math.abs(Number(heal)) }); setHeal(''); }} disabled={!heal || loading}>Heal</button>
        </div>
        {/* Set HP */}
        <div className="control-row">
          <input className="control-input" type="number" min={0} placeholder="Set HP" value={setHpVal} onChange={(e) => setSetHpVal(e.target.value)} />
          <button className="btn btn-sm" onClick={() => { doUpdate({ set_hp: Number(setHpVal) }); setSetHpVal(''); }} disabled={!setHpVal || loading}>Set</button>
        </div>
        {/* Add condition */}
        {availableToAdd.length > 0 && (
          <div className="control-row">
            <select className="control-select" value={addCond} onChange={(e) => setAddCond(e.target.value as Condition)}>
              {availableToAdd.map((c) => <option key={c} value={c}>{c}</option>)}
            </select>
            <button className="btn btn-sm" onClick={() => doUpdate({ add_condition: addCond })} disabled={loading}>+Cond</button>
          </div>
        )}
        {/* Spend action budget */}
        <div className="control-row budget-controls">
          <label className="checkbox-label">
            <input type="checkbox" checked={!p.action_budget.has_action} onChange={() => doUpdate({ spend_action: !p.action_budget.has_action })} />
            <span>Action</span>
          </label>
          <label className="checkbox-label">
            <input type="checkbox" checked={!p.action_budget.has_bonus_action} onChange={() => doUpdate({ spend_bonus_action: !p.action_budget.has_bonus_action })} />
            <span>Bonus</span>
          </label>
          <label className="checkbox-label">
            <input type="checkbox" checked={!p.action_budget.has_reaction} onChange={() => doUpdate({ spend_reaction: !p.action_budget.has_reaction })} />
            <span>Reaction</span>
          </label>
        </div>
      </td>
    </tr>
    {rowError && (
      <tr className="participant-error-row">
        <td colSpan={7}><span className="form-error">{rowError}</span></td>
      </tr>
    )}
    </>
  );
}
