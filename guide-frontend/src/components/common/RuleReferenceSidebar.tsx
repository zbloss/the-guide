import { useState } from 'react';
import { searchRules } from '../../api/documents';
import type { RankedChunk } from '../../api/types';

export function RuleReferenceSidebar() {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<RankedChunk[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!query.trim()) return;
    setLoading(true);
    setError('');
    try {
      const chunks = await searchRules(query);
      setResults(chunks);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className={`rule-sidebar ${open ? 'rule-sidebar-open' : ''}`}>
      <button className="rule-sidebar-toggle btn btn-sm" onClick={() => setOpen((o) => !o)} title="D&D 5e Rules Reference">
        {open ? '✕ Rules' : '📖 Rules'}
      </button>
      {open && (
        <div className="rule-sidebar-panel">
          <h3 className="rule-sidebar-title">Rules Reference</h3>
          <form onSubmit={handleSearch} className="rule-sidebar-form">
            <input
              className="form-input"
              placeholder="Search rules…"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              autoFocus
            />
            <button type="submit" className="btn btn-sm btn-primary" disabled={loading}>
              {loading ? '…' : 'Search'}
            </button>
          </form>
          {error && <div className="form-error" style={{ marginTop: 8 }}>{error}</div>}
          {results.length === 0 && !loading && query && (
            <p className="rule-sidebar-empty">No results found.</p>
          )}
          <div className="rule-sidebar-results">
            {results.map((chunk, i) => (
              <div key={i} className="rule-chunk">
                <div className="rule-chunk-header">
                  <span className="rule-chunk-doc">{chunk.doc_title}</span>
                  {chunk.section_path && (
                    <span className="rule-chunk-section"> › {chunk.section_path}</span>
                  )}
                </div>
                <p className="rule-chunk-content">{chunk.content}</p>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
