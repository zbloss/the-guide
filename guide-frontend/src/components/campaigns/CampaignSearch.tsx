import { useState, useRef } from 'react';
import { useNavigate } from 'react-router-dom';
import { searchCampaign } from '../../api/campaigns';
import { LoadingSpinner } from '../common/LoadingSpinner';
import type { SearchResults } from '../../api/types';

interface CampaignSearchProps {
  campaignId: string;
}

export function CampaignSearch({ campaignId }: CampaignSearchProps) {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchResults | null>(null);
  const [loading, setLoading] = useState(false);
  const [open, setOpen] = useState(false);
  const navigate = useNavigate();
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleInput = (value: string) => {
    setQuery(value);
    if (timerRef.current) clearTimeout(timerRef.current);
    if (!value.trim()) { setResults(null); setOpen(false); return; }
    timerRef.current = setTimeout(async () => {
      setLoading(true);
      try {
        const res = await searchCampaign(campaignId, value);
        setResults(res);
        setOpen(true);
      } catch {
        setResults(null);
      } finally {
        setLoading(false);
      }
    }, 350);
  };

  const total = results
    ? results.characters.length + results.sessions.length + results.events.length
    : 0;

  const handleSelect = (type: string, id: string, sessionId?: string) => {
    setOpen(false);
    setQuery('');
    if (type === 'character') navigate(`/campaigns/${campaignId}/characters/${id}`);
    else if (type === 'session') navigate(`/campaigns/${campaignId}/sessions/${id}`);
    else if (type === 'event' && sessionId) navigate(`/campaigns/${campaignId}/sessions/${sessionId}`);
  };

  return (
    <div className="campaign-search" style={{ position: 'relative', maxWidth: '320px' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
        <input
          className="form-input"
          style={{ flex: 1 }}
          placeholder="🔎 Search campaign…"
          value={query}
          onChange={(e) => handleInput(e.target.value)}
          onFocus={() => results && setOpen(true)}
          onBlur={() => setTimeout(() => setOpen(false), 200)}
        />
        {loading && <LoadingSpinner size={16} />}
      </div>

      {open && results && total > 0 && (
        <div className="search-dropdown" style={{
          position: 'absolute', top: '100%', left: 0, right: 0, zIndex: 100,
          background: 'var(--color-surface, #1e1e2e)', border: '1px solid var(--color-border, #444)',
          borderRadius: '6px', marginTop: '4px', maxHeight: '320px', overflowY: 'auto',
        }}>
          {results.characters.length > 0 && (
            <div>
              <div style={{ padding: '4px 12px', fontSize: '0.75rem', color: '#888', fontWeight: 600 }}>CHARACTERS</div>
              {results.characters.map((r) => (
                <button key={r.id} className="search-result-item" onClick={() => handleSelect(r.type, r.id)}
                  style={{ width: '100%', textAlign: 'left', padding: '6px 12px', background: 'none', border: 'none', cursor: 'pointer', color: 'inherit' }}>
                  {r.label}
                </button>
              ))}
            </div>
          )}
          {results.sessions.length > 0 && (
            <div>
              <div style={{ padding: '4px 12px', fontSize: '0.75rem', color: '#888', fontWeight: 600 }}>SESSIONS</div>
              {results.sessions.map((r) => (
                <button key={r.id} className="search-result-item" onClick={() => handleSelect(r.type, r.id)}
                  style={{ width: '100%', textAlign: 'left', padding: '6px 12px', background: 'none', border: 'none', cursor: 'pointer', color: 'inherit' }}>
                  {r.label}
                </button>
              ))}
            </div>
          )}
          {results.events.length > 0 && (
            <div>
              <div style={{ padding: '4px 12px', fontSize: '0.75rem', color: '#888', fontWeight: 600 }}>EVENTS</div>
              {results.events.map((r) => (
                <button key={r.id} className="search-result-item" onClick={() => handleSelect(r.type, r.id, r.session_id)}
                  style={{ width: '100%', textAlign: 'left', padding: '6px 12px', background: 'none', border: 'none', cursor: 'pointer', color: 'inherit' }}>
                  {r.label}
                </button>
              ))}
            </div>
          )}
        </div>
      )}

      {open && results && total === 0 && (
        <div style={{
          position: 'absolute', top: '100%', left: 0, right: 0, zIndex: 100,
          background: 'var(--color-surface, #1e1e2e)', border: '1px solid var(--color-border, #444)',
          borderRadius: '6px', marginTop: '4px', padding: '8px 12px', color: '#888',
        }}>
          No results for "{results.query}"
        </div>
      )}
    </div>
  );
}
