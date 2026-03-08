import { useState } from 'react';
import { FormField } from '../common/FormField';
import type { CreateSessionEventRequest, EventType, EventSignificance, Character } from '../../api/types';
import { ALL_EVENT_TYPES } from '../../api/types';

interface SessionEventFormProps {
  characters: Character[];
  onSubmit: (data: CreateSessionEventRequest) => Promise<void>;
  onCancel: () => void;
}

export function SessionEventForm({ characters, onSubmit, onCancel }: SessionEventFormProps) {
  const [eventType, setEventType] = useState<EventType>('other');
  const [description, setDescription] = useState('');
  const [significance, setSignificance] = useState<EventSignificance>('minor');
  const [playerVisible, setPlayerVisible] = useState(true);
  const [selectedChars, setSelectedChars] = useState<string[]>([]);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState('');

  const toggleChar = (id: string) => {
    setSelectedChars((prev) => prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]);
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!description.trim()) { setError('Description is required'); return; }
    setSubmitting(true);
    setError('');
    try {
      await onSubmit({
        event_type: eventType,
        description: description.trim(),
        significance,
        is_player_visible: playerVisible,
        involved_character_ids: selectedChars,
      });
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
      setSubmitting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="form">
      {error && <div className="form-error-banner">{error}</div>}

      <div className="form-row">
        <FormField label="Event Type" htmlFor="ev-type">
          <select id="ev-type" className="form-input" value={eventType} onChange={(e) => setEventType(e.target.value as EventType)}>
            {ALL_EVENT_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
          </select>
        </FormField>
        <FormField label="Significance" htmlFor="ev-sig">
          <select id="ev-sig" className="form-input" value={significance} onChange={(e) => setSignificance(e.target.value as EventSignificance)}>
            <option value="minor">Minor</option>
            <option value="moderate">Moderate</option>
            <option value="major">Major</option>
            <option value="critical">Critical</option>
          </select>
        </FormField>
      </div>

      <FormField label="Description" htmlFor="ev-desc">
        <textarea id="ev-desc" className="form-input" value={description} onChange={(e) => setDescription(e.target.value)} rows={3} required />
      </FormField>

      <FormField label="Player Visible">
        <label className="checkbox-label">
          <input type="checkbox" checked={playerVisible} onChange={(e) => setPlayerVisible(e.target.checked)} />
          <span>Visible to players</span>
        </label>
      </FormField>

      {characters.length > 0 && (
        <div className="form-field">
          <label className="form-label">Involved Characters</label>
          <div className="checkbox-group">
            {characters.map((c) => (
              <label key={c.id} className="checkbox-label">
                <input
                  type="checkbox"
                  checked={selectedChars.includes(c.id)}
                  onChange={() => toggleChar(c.id)}
                />
                <span>{c.name}</span>
              </label>
            ))}
          </div>
        </div>
      )}

      <div className="form-actions">
        <button type="submit" className="btn btn-primary" disabled={submitting}>
          {submitting ? 'Adding…' : 'Add Event'}
        </button>
        <button type="button" className="btn" onClick={onCancel} disabled={submitting}>Cancel</button>
      </div>
    </form>
  );
}
