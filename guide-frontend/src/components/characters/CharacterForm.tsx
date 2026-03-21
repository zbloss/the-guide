import { useState } from 'react';
import { FormField } from '../common/FormField';
import type { CreateCharacterRequest, CharacterType } from '../../api/types';

interface ParsedExtras {
  parse_confidence?: number;
  background?: string | null;
  languages?: string[];
  proficiencies?: string[];
  equipment?: string[];
  features_and_traits?: string[];
  personality_traits?: string | null;
  ideals?: string | null;
  bonds?: string | null;
  flaws?: string | null;
  hit_dice?: string | null;
  experience_points?: number | null;
}

interface CharacterFormProps {
  onSubmit: (data: CreateCharacterRequest) => Promise<void>;
  onCancel: () => void;
  initialValues?: Partial<CreateCharacterRequest & ParsedExtras>;
}

export function CharacterForm({ onSubmit, onCancel, initialValues }: CharacterFormProps) {
  const [name, setName] = useState(initialValues?.name ?? '');
  const [charType, setCharType] = useState<CharacterType>(initialValues?.character_type ?? 'pc');
  const [charClass, setCharClass] = useState(initialValues?.class ?? '');
  const [race, setRace] = useState(initialValues?.race ?? '');
  const [level, setLevel] = useState(initialValues?.level ?? 1);
  const [maxHp, setMaxHp] = useState(initialValues?.max_hp ?? 10);
  const [ac, setAc] = useState(initialValues?.armor_class ?? 10);
  const [speed, setSpeed] = useState(initialValues?.speed ?? 30);
  const [str, setStr] = useState(initialValues?.ability_scores?.strength ?? 10);
  const [dex, setDex] = useState(initialValues?.ability_scores?.dexterity ?? 10);
  const [con, setCon] = useState(initialValues?.ability_scores?.constitution ?? 10);
  const [int, setInt] = useState(initialValues?.ability_scores?.intelligence ?? 10);
  const [wis, setWis] = useState(initialValues?.ability_scores?.wisdom ?? 10);
  const [cha, setCha] = useState(initialValues?.ability_scores?.charisma ?? 10);
  const [backstory, setBackstory] = useState(initialValues?.backstory_text ?? '');
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
      {initialValues?.parse_confidence !== undefined && initialValues.parse_confidence < 0.7 && (
        <div className="form-warning-banner">
          Low parse confidence ({(initialValues.parse_confidence * 100).toFixed(0)}%) — please review the pre-filled values.
        </div>
      )}

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

      {/* Read-only section for extra parsed data from PDF import */}
      {initialValues && (
        initialValues.background ||
        (initialValues.languages && initialValues.languages.length > 0) ||
        (initialValues.proficiencies && initialValues.proficiencies.length > 0) ||
        (initialValues.equipment && initialValues.equipment.length > 0) ||
        (initialValues.features_and_traits && initialValues.features_and_traits.length > 0) ||
        initialValues.personality_traits ||
        initialValues.ideals ||
        initialValues.bonds ||
        initialValues.flaws
      ) && (
        <div className="form-section-label" style={{ marginTop: 16 }}>Parsed Character Info (read-only)</div>
      )}
      {initialValues?.background && (
        <div className="form-parsed-row"><strong>Background:</strong> {initialValues.background}</div>
      )}
      {initialValues?.hit_dice && (
        <div className="form-parsed-row"><strong>Hit Dice:</strong> {initialValues.hit_dice}</div>
      )}
      {initialValues?.experience_points != null && (
        <div className="form-parsed-row"><strong>Experience Points:</strong> {initialValues.experience_points}</div>
      )}
      {initialValues?.languages && initialValues.languages.length > 0 && (
        <div className="form-parsed-row"><strong>Languages:</strong> {initialValues.languages.join(', ')}</div>
      )}
      {initialValues?.proficiencies && initialValues.proficiencies.length > 0 && (
        <div className="form-parsed-row"><strong>Proficiencies:</strong> {initialValues.proficiencies.join(', ')}</div>
      )}
      {initialValues?.equipment && initialValues.equipment.length > 0 && (
        <div className="form-parsed-row">
          <strong>Equipment:</strong>
          <ul style={{ margin: '4px 0 0 16px', padding: 0 }}>
            {initialValues.equipment.map((item, i) => <li key={i}>{item}</li>)}
          </ul>
        </div>
      )}
      {initialValues?.features_and_traits && initialValues.features_and_traits.length > 0 && (
        <div className="form-parsed-row">
          <strong>Features &amp; Traits:</strong>
          <ul style={{ margin: '4px 0 0 16px', padding: 0 }}>
            {initialValues.features_and_traits.map((f, i) => <li key={i}>{f}</li>)}
          </ul>
        </div>
      )}
      {initialValues?.personality_traits && (
        <div className="form-parsed-row"><strong>Personality Traits:</strong> {initialValues.personality_traits}</div>
      )}
      {initialValues?.ideals && (
        <div className="form-parsed-row"><strong>Ideals:</strong> {initialValues.ideals}</div>
      )}
      {initialValues?.bonds && (
        <div className="form-parsed-row"><strong>Bonds:</strong> {initialValues.bonds}</div>
      )}
      {initialValues?.flaws && (
        <div className="form-parsed-row"><strong>Flaws:</strong> {initialValues.flaws}</div>
      )}

      <div className="form-actions">
        <button type="submit" className="btn btn-primary" disabled={submitting}>
          {submitting ? 'Creating…' : 'Create Character'}
        </button>
        <button type="button" className="btn" onClick={onCancel} disabled={submitting}>Cancel</button>
      </div>
    </form>
  );
}
