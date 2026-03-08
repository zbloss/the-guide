import { useState } from 'react';
import { FormField } from '../common/FormField';
import type { CreateSessionRequest } from '../../api/types';

interface SessionFormProps {
  onSubmit: (data: CreateSessionRequest) => Promise<void>;
  onCancel: () => void;
}

export function SessionForm({ onSubmit, onCancel }: SessionFormProps) {
  const [title, setTitle] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setSubmitting(true);
    setError('');
    try {
      await onSubmit({ title: title.trim() || undefined });
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : String(err));
      setSubmitting(false);
    }
  };

  return (
    <form onSubmit={handleSubmit} className="form">
      {error && <div className="form-error-banner">{error}</div>}
      <FormField label="Title (optional)" htmlFor="s-title">
        <input
          id="s-title"
          className="form-input"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          placeholder="e.g., The Dragon's Lair"
        />
      </FormField>
      <div className="form-actions">
        <button type="submit" className="btn btn-primary" disabled={submitting}>
          {submitting ? 'Creating…' : 'Create Session'}
        </button>
        <button type="button" className="btn" onClick={onCancel} disabled={submitting}>Cancel</button>
      </div>
    </form>
  );
}
