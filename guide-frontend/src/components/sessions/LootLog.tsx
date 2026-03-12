import { useState } from 'react';
import { useApi } from '../../hooks/useApi';
import { listLoot, createLootItem, deleteLootItem } from '../../api/loot';
import { ConfirmButton } from '../common/ConfirmButton';
import { LoadingSpinner } from '../common/LoadingSpinner';
import { ErrorBanner } from '../common/ErrorBanner';
import type { LootItem, CreateLootItemRequest } from '../../api/types';

interface LootLogProps {
  campaignId: string;
  sessionId: string;
}

export function LootLog({ campaignId, sessionId }: LootLogProps) {
  const { data: items, loading, error, refetch } = useApi<LootItem[]>(
    () => listLoot(campaignId, sessionId),
    [campaignId, sessionId],
  );

  const [showForm, setShowForm] = useState(false);
  const [name, setName] = useState('');
  const [itemType, setItemType] = useState('misc');
  const [quantity, setQuantity] = useState(1);
  const [valueGp, setValueGp] = useState(0);
  const [notes, setNotes] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [formError, setFormError] = useState('');

  const totalGp = (items ?? []).reduce((sum, i) => sum + i.value_gp * i.quantity, 0);

  const handleAdd = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) { setFormError('Name is required'); return; }
    setSubmitting(true);
    setFormError('');
    try {
      const req: CreateLootItemRequest = {
        name: name.trim(),
        item_type: itemType as CreateLootItemRequest['item_type'],
        quantity,
        value_gp: valueGp,
        notes: notes.trim() || undefined,
      };
      await createLootItem(campaignId, sessionId, req);
      setName(''); setItemType('misc'); setQuantity(1); setValueGp(0); setNotes('');
      setShowForm(false);
      refetch();
    } catch (err: unknown) {
      setFormError(err instanceof Error ? err.message : String(err));
    } finally {
      setSubmitting(false);
    }
  };

  const handleDelete = async (itemId: string) => {
    try {
      await deleteLootItem(campaignId, sessionId, itemId);
      refetch();
    } catch {
      // ignored - item stays visible
    }
  };

  if (loading) return <LoadingSpinner />;
  if (error) return <ErrorBanner message={error} />;

  return (
    <div className="loot-log">
      <div className="section-header">
        <h3>Loot & Treasure</h3>
        <div className="btn-group">
          {totalGp > 0 && <span className="loot-total">Total: {totalGp.toLocaleString()} gp</span>}
          <button className="btn btn-sm btn-primary" onClick={() => setShowForm((s) => !s)}>
            {showForm ? 'Cancel' : '+ Add Item'}
          </button>
        </div>
      </div>

      {showForm && (
        <form onSubmit={handleAdd} className="form loot-form">
          {formError && <div className="form-error-banner">{formError}</div>}
          <div className="form-row">
            <div className="form-field" style={{ flex: 2 }}>
              <label className="form-label">Name</label>
              <input className="form-input" value={name} onChange={(e) => setName(e.target.value)} placeholder="Sword of Flames" required />
            </div>
            <div className="form-field">
              <label className="form-label">Type</label>
              <select className="form-input" value={itemType} onChange={(e) => setItemType(e.target.value)}>
                <option value="misc">Misc</option>
                <option value="weapon">Weapon</option>
                <option value="armor">Armor</option>
                <option value="magic">Magic</option>
                <option value="currency">Currency</option>
              </select>
            </div>
            <div className="form-field" style={{ width: '80px' }}>
              <label className="form-label">Qty</label>
              <input className="form-input" type="number" min={1} value={quantity} onChange={(e) => setQuantity(Number(e.target.value))} />
            </div>
            <div className="form-field" style={{ width: '100px' }}>
              <label className="form-label">Value (gp)</label>
              <input className="form-input" type="number" min={0} step={0.01} value={valueGp} onChange={(e) => setValueGp(Number(e.target.value))} />
            </div>
          </div>
          <div className="form-field">
            <label className="form-label">Notes</label>
            <input className="form-input" value={notes} onChange={(e) => setNotes(e.target.value)} placeholder="Optional notes…" />
          </div>
          <div className="form-actions">
            <button type="submit" className="btn btn-primary btn-sm" disabled={submitting}>
              {submitting ? 'Adding…' : 'Add Item'}
            </button>
          </div>
        </form>
      )}

      {(!items || items.length === 0) && !showForm ? (
        <p className="empty-state">No loot recorded for this session.</p>
      ) : items && items.length > 0 ? (
        <table className="data-table">
          <thead>
            <tr>
              <th>Item</th>
              <th>Type</th>
              <th>Qty</th>
              <th>Value (gp)</th>
              <th>Notes</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {items.map((item) => (
              <tr key={item.id}>
                <td>{item.name}</td>
                <td>{item.item_type}</td>
                <td>{item.quantity}</td>
                <td>{item.value_gp > 0 ? item.value_gp.toLocaleString() : '—'}</td>
                <td>{item.notes ?? '—'}</td>
                <td>
                  <ConfirmButton label="Delete" variant="danger" onConfirm={() => handleDelete(item.id)} />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : null}
    </div>
  );
}
