import { useState } from 'react';
import { FormField } from '../common/FormField';
import type { CreateCharacterRequest, CharacterType } from '../../api/types';

interface CharacterFormProps {
  onSubmit: (data: CreateCharacterRequest) => Promise<void>;
  onCancel: () => void;
}

export function CharacterForm({ onSubmit, onCancel }: CharacterFormProps) {
  const [name, setName] = useState('');
  const [charType, setCharType] = useState<CharacterType>('pc');
  const [charClass, setCharClass] = useState('');
  const [race, setRace] = useState('');
  const [level, setLevel] = useState(1);
  const [maxHp, setMaxHp] = useState(10);
  const [ac, setAc] = useState(10);
  const [speed, setSpeed] = useState(30);
  const [str, setStr] = useState(10);
  const [dex, setDex] = useState(10);
  const [con, setCon] = useState(10);
  const [int, setInt] = useState(10);
  const [wis, setWis] = useState(10);
  const [cha, setCha] = useState(10);
  const [backstory, setBackstory] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) { setError('Name is required'); return; }
    setSubmitting(true);
    setError('');
    try {
      await onSubmit({
        name: name.trim(),
        character_type: charType,
        class: charClass.trim() || undefined,
        race: race.trim() || undefined,
        level,
        max_hp: maxHp,
        armor_class: ac,
        speed,
        ability_scores: { strength: str, dexterity: dex, constitution: con, intelligence: int, wisdom: wis, charisma: cha },
        backstory_text: backstory.trim() || undefined,
      });
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
      setSubmitting(false);
    }
  };

  const numInput = (value: number, onChange: (v: number) => void) => (
    <input
      type="number"
      className="form-input form-input-num"
      value={value}
      min={0}
      onChange={(e) => onChange(Number(e.target.value))}
    />
  );

  return (
    <form onSubmit={handleSubmit} className="form">
      {error && <div className="form-error-banner">{error}</div>}

      <FormField label="Name" htmlFor="ch-name">
        <input id="ch-name" className="form-input" value={name} onChange={(e) => setName(e.target.value)} required />
      </FormField>

      <div className="form-row">
        <FormField label="Type" htmlFor="ch-type">
          <select id="ch-type" className="form-input" value={charType} onChange={(e) => setCharType(e.target.value as CharacterType)}>
            <option value="pc">PC</option>
            <option value="npc">NPC</option>
            <option value="monster">Monster</option>
          </select>
        </FormField>
        <FormField label="Class" htmlFor="ch-class">
          <input id="ch-class" className="form-input" value={charClass} onChange={(e) => setCharClass(e.target.value)} placeholder="Fighter" />
        </FormField>
        <FormField label="Race" htmlFor="ch-race">
          <input id="ch-race" className="form-input" value={race} onChange={(e) => setRace(e.target.value)} placeholder="Human" />
        </FormField>
      </div>

      <div className="form-row">
        <FormField label="Level">{numInput(level, setLevel)}</FormField>
        <FormField label="Max HP">{numInput(maxHp, setMaxHp)}</FormField>
        <FormField label="AC">{numInput(ac, setAc)}</FormField>
        <FormField label="Speed">{numInput(speed, setSpeed)}</FormField>
      </div>

      <div className="form-section-label">Ability Scores</div>
      <div className="form-row ability-scores">
        <FormField label="STR">{numInput(str, setStr)}</FormField>
        <FormField label="DEX">{numInput(dex, setDex)}</FormField>
        <FormField label="CON">{numInput(con, setCon)}</FormField>
        <FormField label="INT">{numInput(int, setInt)}</FormField>
        <FormField label="WIS">{numInput(wis, setWis)}</FormField>
        <FormField label="CHA">{numInput(cha, setCha)}</FormField>
      </div>

      <FormField label="Backstory" htmlFor="ch-bs">
        <textarea id="ch-bs" className="form-input" value={backstory} onChange={(e) => setBackstory(e.target.value)} rows={4} placeholder="Character backstory…" />
      </FormField>

      <div className="form-actions">
        <button type="submit" className="btn btn-primary" disabled={submitting}>
          {submitting ? 'Creating…' : 'Create Character'}
        </button>
        <button type="button" className="btn" onClick={onCancel} disabled={submitting}>Cancel</button>
      </div>
    </form>
  );
}
