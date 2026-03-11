import { useState } from 'react';
import { generateEncounter, createEncounter } from '../../api/encounters';
import { LoadingSpinner } from '../common/LoadingSpinner';
import type { GeneratedEncounter } from '../../api/types';

interface GenerateEncounterPanelProps {
  campaignId: string;
  onSaved?: () => void;
}

export function GenerateEncounterPanel({ campaignId, onSaved }: GenerateEncounterPanelProps) {
  const [context, setContext] = useState('');
  const [partyLevel, setPartyLevel] = useState(1);
  const [generating, setGenerating] = useState(false);
  const [result, setResult] = useState<GeneratedEncounter | null>(null);
  const [error, setError] = useState('');
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);

  const handleGenerate = async () => {
    if (!context.trim()) { setError('Context is required'); return; }
    setGenerating(true);
    setError('');
    setResult(null);
    setSaved(false);
    try {
      const enc = await generateEncounter(campaignId, { context: context.trim(), party_level: partyLevel });
      setResult(enc);
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setGenerating(false);
    }
  };

  const handleSave = async () => {
    if (!result) return;
    setSaving(true);
    setError('');
    try {
      await createEncounter(campaignId, {
        name: result.title,
        description: `${result.description}\n\n${result.narrative_hook}`,
        participant_character_ids: [],
      });
      setSaved(true);
      onSaved?.();
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="generate-panel">
      <h3>Generate Encounter with AI</h3>
      {error && <div className="form-error-banner">{error}</div>}

      <div className="form-field">
        <label className="form-label">Context</label>
        <textarea
          className="form-input"
          value={context}
          onChange={(e) => setContext(e.target.value)}
          rows={3}
          placeholder="Describe the setting, current quest, or situation…"
        />
      </div>

      <div className="form-field">
        <label className="form-label">Party Level</label>
        <input
          type="number"
          className="form-input form-input-num"
          value={partyLevel}
          min={1}
          max={20}
          onChange={(e) => setPartyLevel(Number(e.target.value))}
        />
      </div>

      <button className="btn btn-primary" onClick={handleGenerate} disabled={generating}>
        {generating ? <><LoadingSpinner size={14} /> Generating…</> : 'Generate Encounter'}
      </button>

      {result && (
        <div className="generated-encounter">
          <h4>{result.title}</h4>
          <p className="badge badge-info">{result.encounter_type}</p>
          <p>{result.description}</p>
          <p className="narrative-hook">"{result.narrative_hook}"</p>

          {result.suggested_enemies.length > 0 && (
            <div>
              <h5>Enemies</h5>
              <table className="data-table">
                <thead><tr><th>Name</th><th>Count</th><th>CR</th></tr></thead>
                <tbody>
                  {result.suggested_enemies.map((e, i) => (
                    <tr key={i}>
                      <td>{e.name}</td>
                      <td>{e.count}</td>
                      <td>{e.cr ?? '—'}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}

          <div className="form-actions" style={{ marginTop: 12 }}>
            {saved ? (
              <span className="badge badge-success">Encounter saved ✓</span>
            ) : (
              <button className="btn btn-primary" onClick={handleSave} disabled={saving}>
                {saving ? <><LoadingSpinner size={14} /> Saving…</> : 'Save Encounter'}
              </button>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
