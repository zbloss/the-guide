import { useState } from 'react';
import { searchCampaignDoc } from '../../api/documents';
import type { CampaignDocument, RankedChunk } from '../../api/types';

interface Props {
  campaignId: string;
  documents: CampaignDocument[];
}

export function DocumentChunkSearch({ campaignId, documents }: Props) {
  const [selectedDocId, setSelectedDocId] = useState('');
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<RankedChunk[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [searched, setSearched] = useState(false);

  const ingestedDocs = documents.filter((d) => d.ingestion_status === 'completed');

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!query.trim() || !selectedDocId) return;
    setLoading(true);
    setError('');
    setSearched(false);
    try {
      const chunks = await searchCampaignDoc(campaignId, selectedDocId, query);
      setResults(chunks);
      setSearched(true);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  if (ingestedDocs.length === 0) return null;

  return (
    <div className="doc-chunk-search page-section" style={{ marginTop: 16 }}>
      <h3>Search Document Chunks</h3>
      <form onSubmit={handleSearch} style={{ display: 'flex', gap: 8, flexWrap: 'wrap', marginBottom: 12 }}>
        <select
          className="form-input"
          style={{ flex: '0 0 auto', minWidth: 180 }}
          value={selectedDocId}
          onChange={(e) => setSelectedDocId(e.target.value)}
        >
          <option value="">Select document…</option>
          {ingestedDocs.map((d) => (
            <option key={d.id} value={d.id}>{d.filename}</option>
          ))}
        </select>
        <input
          className="form-input"
          style={{ flex: 1, minWidth: 200 }}
          placeholder="Search within document…"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <button type="submit" className="btn btn-primary btn-sm" disabled={!selectedDocId || !query.trim() || loading}>
          {loading ? 'Searching…' : 'Search'}
        </button>
      </form>
      {error && <div className="form-error">{error}</div>}
      {searched && results.length === 0 && <p className="text-muted">No matching chunks found.</p>}
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
  );
}
