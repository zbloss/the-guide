import { useState, useEffect, useCallback } from 'react';
import { listHomebrewRules, createHomebrewRule, deleteHomebrewRule } from '../../api/homebrew';
import { LoadingSpinner } from '../common/LoadingSpinner';
import { Badge } from '../common/Badge';
import type { HomebrewRule } from '../../api/types';

const CATEGORIES = ['general', 'combat', 'magic', 'exploration', 'social'] as const;

interface HomebrewRuleListProps {
  campaignId: string;
}

export function HomebrewRuleList({ campaignId }: HomebrewRuleListProps) {
  const [rules, setRules] = useState<HomebrewRule[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [expanded, setExpanded] = useState(false);

  const [showForm, setShowForm] = useState(false);
  const [formTitle, setFormTitle] = useState('');
  const [formDescription, setFormDescription] = useState('');
  const [formCategory, setFormCategory] = useState<string>('general');
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState('');

  const load = useCallback(async () => {
    try {
      const data = await listHomebrewRules(campaignId);
      setRules(data);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [campaignId]);

  useEffect(() => { void load(); }, [load]);

  const handleCreate = async () => {
    if (!formTitle.trim() || !formDescription.trim()) {
      setFormError('Title and description are required.');
      return;
    }
    setSaving(true);
    setFormError('');
    try {
      await createHomebrewRule(campaignId, {
        title: formTitle.trim(),
        description: formDescription.trim(),
        category: formCategory,
      });
      setFormTitle('');
      setFormDescription('');
      setFormCategory('general');
      setShowForm(false);
      await load();
    } catch (e: unknown) {
      setFormError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (id: string) => {
    try {
      await deleteHomebrewRule(campaignId, id);
      setRules((prev) => prev.filter((r) => r.id !== id));
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div className="homebrew-panel">
      <div className="section-header">
        <h3
          style={{ cursor: 'pointer', userSelect: 'none' }}
          onClick={() => setExpanded((e) => !e)}
        >
          {expanded ? '▼' : '▶'} Homebrew Rules {rules.length > 0 && <span className="badge badge-info">{rules.length}</span>}
        </h3>
        <button className="btn btn-sm btn-primary" onClick={() => { setExpanded(true); setShowForm((s) => !s); }}>
          {showForm ? 'Cancel' : '+ Add Rule'}
        </button>
      </div>

      {error && <div className="form-error-banner">{error}</div>}

      {showForm && (
        <div className="homebrew-form card" style={{ marginBottom: '0.75rem' }}>
          {formError && <div className="form-error-banner">{formError}</div>}
          <div className="form-field">
            <label className="form-label">Title</label>
            <input
              className="form-input"
              value={formTitle}
              onChange={(e) => setFormTitle(e.target.value)}
              placeholder="Rule title"
            />
          </div>
          <div className="form-field">
            <label className="form-label">Description</label>
            <textarea
              className="form-input"
              value={formDescription}
              onChange={(e) => setFormDescription(e.target.value)}
              placeholder="Describe how this rule works..."
              rows={3}
            />
          </div>
          <div className="form-field">
            <label className="form-label">Category</label>
            <select className="form-input" value={formCategory} onChange={(e) => setFormCategory(e.target.value)}>
              {CATEGORIES.map((cat) => (
                <option key={cat} value={cat}>{cat.charAt(0).toUpperCase() + cat.slice(1)}</option>
              ))}
            </select>
          </div>
          <div className="form-actions">
            <button className="btn btn-primary btn-sm" onClick={handleCreate} disabled={saving}>
              {saving ? <><LoadingSpinner size={14} /> Saving…</> : 'Add Rule'}
            </button>
            <button className="btn btn-sm" onClick={() => setShowForm(false)} disabled={saving}>Cancel</button>
          </div>
        </div>
      )}

      {expanded && (
        loading ? <LoadingSpinner /> : (
          rules.length === 0 ? (
            <p className="empty-state">No homebrew rules yet. Add one above.</p>
          ) : (
            <ul className="homebrew-list">
              {rules.map((rule) => (
                <li key={rule.id} className="homebrew-item card">
                  <div className="homebrew-item-header">
                    <strong>{rule.title}</strong>
                    <Badge label={rule.category} variant="info" />
                    <button
                      className="btn btn-sm btn-danger"
                      style={{ marginLeft: 'auto' }}
                      onClick={() => void handleDelete(rule.id)}
                    >
                      Delete
                    </button>
                  </div>
                  <p className="homebrew-item-desc">{rule.description}</p>
                </li>
              ))}
            </ul>
          )
        )
      )}
    </div>
  );
}
