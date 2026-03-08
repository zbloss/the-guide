import { useState } from 'react';
import { FormField } from '../common/FormField';
import type { CreateCampaignRequest, GameSystem } from '../../api/types';

interface CampaignFormProps {
  onSubmit: (data: CreateCampaignRequest) => Promise<void>;
  onCancel: () => void;
}

export function CampaignForm({ onSubmit, onCancel }: CampaignFormProps) {
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [gameSystem, setGameSystem] = useState<GameSystem>('dnd5e');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) { setError('Name is required'); return; }
    setSubmitting(true);
    setError('');
    try {
      await onSubmit({ name: name.trim(), description: description.trim() || undefined, game_system: gameSystem });
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="form">
      {error && <div className="form-error-banner">{error}</div>}

      <FormField label="Name" htmlFor="c-name">
        <input
          id="c-name"
          className="form-input"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="My Campaign"
          required
        />
      </FormField>

      <FormField label="Description" htmlFor="c-desc">
        <textarea
          id="c-desc"
          className="form-input"
          value={description}
          onChange={(e) => setDescription(e.target.value)}
          rows={3}
          placeholder="Optional description…"
        />
      </FormField>

      <FormField label="Game System" htmlFor="c-sys">
        <select
          id="c-sys"
          className="form-input"
          value={gameSystem}
          onChange={(e) => setGameSystem(e.target.value as GameSystem)}
        >
          <option value="dnd5e">D&D 5e</option>
          <option value="pathfinder2e">Pathfinder 2e</option>
          <option value="custom">Custom</option>
        </select>
      </FormField>

      <div className="form-actions">
        <button type="submit" className="btn btn-primary" disabled={submitting}>
          {submitting ? 'Creating…' : 'Create Campaign'}
        </button>
        <button type="button" className="btn" onClick={onCancel} disabled={submitting}>
          Cancel
        </button>
      </div>
    </form>
  );
}
