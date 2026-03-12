import { useState, useEffect, useCallback } from 'react';
import { listWebhooks, createWebhook, deleteWebhook } from '../../api/webhooks';
import type { CampaignWebhook } from '../../api/types';
import { LoadingSpinner } from '../common/LoadingSpinner';

const AVAILABLE_EVENTS = [
  { key: 'session_start', label: 'Session Start' },
  { key: 'session_end', label: 'Session End' },
  { key: 'encounter_end', label: 'Encounter End' },
];

interface WebhookManagerProps {
  campaignId: string;
}

export function WebhookManager({ campaignId }: WebhookManagerProps) {
  const [webhooks, setWebhooks] = useState<CampaignWebhook[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [expanded, setExpanded] = useState(false);

  const [showForm, setShowForm] = useState(false);
  const [formUrl, setFormUrl] = useState('');
  const [formEvents, setFormEvents] = useState<string[]>(['session_start', 'session_end', 'encounter_end']);
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState('');

  const load = useCallback(async () => {
    try {
      const data = await listWebhooks(campaignId);
      setWebhooks(data);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, [campaignId]);

  useEffect(() => { void load(); }, [load]);

  const toggleEvent = (eventKey: string) => {
    setFormEvents((prev) =>
      prev.includes(eventKey)
        ? prev.filter((e) => e !== eventKey)
        : [...prev, eventKey],
    );
  };

  const handleCreate = async () => {
    if (!formUrl.trim()) {
      setFormError('Webhook URL is required.');
      return;
    }
    if (formEvents.length === 0) {
      setFormError('Select at least one event.');
      return;
    }
    setSaving(true);
    setFormError('');
    try {
      await createWebhook(campaignId, {
        url: formUrl.trim(),
        events: formEvents,
      });
      setFormUrl('');
      setFormEvents(['session_start', 'session_end', 'encounter_end']);
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
      await deleteWebhook(campaignId, id);
      setWebhooks((prev) => prev.filter((w) => w.id !== id));
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  return (
    <div className="webhook-panel" style={{ marginBottom: 16 }}>
      <div className="section-header" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
        <h3
          style={{ cursor: 'pointer', userSelect: 'none', margin: 0 }}
          onClick={() => setExpanded((e) => !e)}
        >
          {expanded ? '▼' : '▶'} Webhooks{' '}
          {webhooks.length > 0 && (
            <span className="badge badge-info">{webhooks.length}</span>
          )}
        </h3>
        <button
          className="btn btn-sm btn-primary"
          onClick={() => { setExpanded(true); setShowForm((s) => !s); }}
        >
          {showForm ? 'Cancel' : '+ Add Webhook'}
        </button>
      </div>

      {error && <div className="form-error-banner">{error}</div>}

      {showForm && (
        <div className="card" style={{ marginTop: 8, marginBottom: 8, padding: '10px 12px' }}>
          {formError && <div className="form-error-banner">{formError}</div>}
          <div className="form-field">
            <label className="form-label">Webhook URL</label>
            <input
              className="form-input"
              value={formUrl}
              onChange={(e) => setFormUrl(e.target.value)}
              placeholder="https://discord.com/api/webhooks/..."
            />
          </div>
          <div className="form-field">
            <label className="form-label">Events</label>
            <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap', marginTop: 4 }}>
              {AVAILABLE_EVENTS.map((ev) => (
                <label key={ev.key} className="checkbox-label" style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                  <input
                    type="checkbox"
                    checked={formEvents.includes(ev.key)}
                    onChange={() => toggleEvent(ev.key)}
                  />
                  <span>{ev.label}</span>
                </label>
              ))}
            </div>
          </div>
          <div className="form-actions">
            <button className="btn btn-primary btn-sm" onClick={() => void handleCreate()} disabled={saving}>
              {saving ? <><LoadingSpinner size={14} /> Saving…</> : 'Add Webhook'}
            </button>
            <button className="btn btn-sm" onClick={() => setShowForm(false)} disabled={saving}>
              Cancel
            </button>
          </div>
        </div>
      )}

      {expanded && (
        loading ? <LoadingSpinner /> : (
          webhooks.length === 0 ? (
            <p className="empty-state">No webhooks registered. Add one to receive Discord notifications.</p>
          ) : (
            <ul className="homebrew-list" style={{ marginTop: 8 }}>
              {webhooks.map((webhook) => (
                <li key={webhook.id} className="homebrew-item card" style={{ padding: '8px 12px' }}>
                  <div className="homebrew-item-header" style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                    <div style={{ flex: 1, overflow: 'hidden' }}>
                      <div style={{ fontSize: 13, fontWeight: 600, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                        {webhook.url}
                      </div>
                      <div style={{ display: 'flex', gap: 4, marginTop: 4, flexWrap: 'wrap' }}>
                        {webhook.events.map((ev) => (
                          <span key={ev} className="badge badge-info" style={{ fontSize: 11 }}>
                            {AVAILABLE_EVENTS.find((a) => a.key === ev)?.label ?? ev}
                          </span>
                        ))}
                      </div>
                    </div>
                    <button
                      className="btn btn-sm btn-danger"
                      onClick={() => void handleDelete(webhook.id)}
                      style={{ flexShrink: 0 }}
                    >
                      Delete
                    </button>
                  </div>
                </li>
              ))}
            </ul>
          )
        )
      )}
    </div>
  );
}
