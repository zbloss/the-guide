import { useState } from 'react';
import { FormField } from '../common/FormField';
import type { CreateEncounterRequest, Session, Character } from '../../api/types';

interface EncounterFormProps {
  sessions: Session[];
  characters: Character[];
  onSubmit: (data: CreateEncounterRequest) => Promise<void>;
  onCancel: () => void;
}

export function EncounterForm({ sessions, characters, onSubmit, onCancel }: EncounterFormProps) {
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [sessionId, setSessionId] = useState('');
  const [selectedChars, setSelectedChars] = useState<string[]>([]);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState('');

  const toggleChar = (id: string) => {
    setSelectedChars((prev) => prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) { setError('Name is required'); return; }
    if (selectedChars.length === 0) { setError('Select at least one participant'); return; }
    setSubmitting(true);
    setError('');
    try {
      await onSubmit({
        name: name.trim(),
        description: description.trim() || undefined,
        session_id: sessionId || undefined,
        participant_character_ids: selectedChars,
      });
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
      setSubmitting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="form">
      {error && <div className="form-error-banner">{error}</div>}

      <FormField label="Name" htmlFor="enc-name">
        <input id="enc-name" className="form-input" value={name} onChange={(e) => setName(e.target.value)} required />
      </FormField>

      <FormField label="Description" htmlFor="enc-desc">
        <textarea id="enc-desc" className="form-input" value={description} onChange={(e) => setDescription(e.target.value)} rows={2} />
      </FormField>

      {sessions.length > 0 && (
        <FormField label="Session (optional)" htmlFor="enc-session">
          <select id="enc-session" className="form-input" value={sessionId} onChange={(e) => setSessionId(e.target.value)}>
            <option value="">— None —</option>
            {sessions.map((s) => (
              <option key={s.id} value={s.id}>
                Session {s.session_number}{s.title ? `: ${s.title}` : ''}
              </option>
            ))}
          </select>
        </FormField>
      )}

      <div className="form-field">
        <label className="form-label">Participants</label>
        <div className="checkbox-group">
          {characters.map((c) => (
            <label key={c.id} className="checkbox-label">
              <input
                type="checkbox"
                checked={selectedChars.includes(c.id)}
                onChange={() => toggleChar(c.id)}
              />
              <span>{c.name} ({c.character_type})</span>
            </label>
          ))}
        </div>
      </div>

      <div className="form-actions">
        <button type="submit" className="btn btn-primary" disabled={submitting}>
          {submitting ? 'Creating…' : 'Create Encounter'}
        </button>
        <button type="button" className="btn" onClick={onCancel} disabled={submitting}>Cancel</button>
      </div>
    </form>
  );
}
