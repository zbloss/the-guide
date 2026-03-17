import { useState, useRef, useEffect } from 'react';
import { useParams } from 'react-router-dom';
import { useApi } from '../hooks/useApi';
import { listCharacters } from '../api/characters';
import { listRelationships, createRelationship, deleteRelationship } from '../api/relationships';
import { Modal } from '../components/common/Modal';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import type { Character, CharacterRelationship, CreateRelationshipRequest } from '../api/types';
import { RELATIONSHIP_TYPES } from '../api/types';

interface NodePos { x: number; y: number; vx: number; vy: number; }

function useForceLayout(nodeIds: string[], links: { source: string; target: string }[], width: number, height: number) {
  const [positions, setPositions] = useState<Record<string, NodePos>>({});

  useEffect(() => {
    if (!nodeIds.length) return;
    const pos: Record<string, NodePos> = {};
    nodeIds.forEach((id, i) => {
      const angle = (i / nodeIds.length) * 2 * Math.PI;
      const r = Math.min(width, height) * 0.35;
      pos[id] = { x: width / 2 + r * Math.cos(angle), y: height / 2 + r * Math.sin(angle), vx: 0, vy: 0 };
    });

    let frame: number;
    let iter = 0;
    const simulate = () => {
      if (iter++ > 300) return;
      const next = { ...pos };
      // Repulsion
      for (const a of nodeIds) {
        for (const b of nodeIds) {
          if (a === b) continue;
          const dx = next[a].x - next[b].x;
          const dy = next[a].y - next[b].y;
          const dist = Math.sqrt(dx * dx + dy * dy) || 1;
          const force = 3000 / (dist * dist);
          next[a].vx += (dx / dist) * force;
          next[a].vy += (dy / dist) * force;
        }
      }
      // Attraction along links
      for (const link of links) {
        const a = next[link.source];
        const b = next[link.target];
        if (!a || !b) continue;
        const dx = b.x - a.x;
        const dy = b.y - a.y;
        const dist = Math.sqrt(dx * dx + dy * dy) || 1;
        const force = (dist - 150) * 0.05;
        a.vx += (dx / dist) * force;
        a.vy += (dy / dist) * force;
        b.vx -= (dx / dist) * force;
        b.vy -= (dy / dist) * force;
      }
      // Center gravity
      for (const id of nodeIds) {
        next[id].vx += (width / 2 - next[id].x) * 0.005;
        next[id].vy += (height / 2 - next[id].y) * 0.005;
        // Damping
        next[id].vx *= 0.85;
        next[id].vy *= 0.85;
        next[id].x = Math.max(40, Math.min(width - 40, next[id].x + next[id].vx));
        next[id].y = Math.max(40, Math.min(height - 40, next[id].y + next[id].vy));
        pos[id] = next[id];
      }
      setPositions({ ...pos });
      frame = requestAnimationFrame(simulate);
    };
    frame = requestAnimationFrame(simulate);
    return () => cancelAnimationFrame(frame);
  }, [nodeIds.join(','), links.length, width, height]); // eslint-disable-line react-hooks/exhaustive-deps

  return positions;
}

export function RelationshipMapPage() {
  const { campaignId } = useParams<{ campaignId: string }>();
  const { data: characters, loading: charsLoading, error: charsError } = useApi<Character[]>(
    () => listCharacters(campaignId!),
    [campaignId],
  );
  const { data: relationships, loading: relsLoading, error: relsError, refetch: refetchRels } = useApi<CharacterRelationship[]>(
    () => listRelationships(campaignId!),
    [campaignId],
  );

  const [selectedCharId, setSelectedCharId] = useState<string | null>(null);
  const [showAddModal, setShowAddModal] = useState(false);
  const [addError, setAddError] = useState('');

  // Add relationship form state
  const [fromId, setFromId] = useState('');
  const [toId, setToId] = useState('');
  const [relType, setRelType] = useState('ally');
  const [relNotes, setRelNotes] = useState('');
  const [submitting, setSubmitting] = useState(false);

  const svgRef = useRef<SVGSVGElement>(null);
  const [svgSize, setSvgSize] = useState({ width: 800, height: 500 });

  useEffect(() => {
    const update = () => {
      if (svgRef.current) {
        const rect = svgRef.current.parentElement?.getBoundingClientRect();
        if (rect) setSvgSize({ width: rect.width || 800, height: 500 });
      }
    };
    update();
    window.addEventListener('resize', update);
    return () => window.removeEventListener('resize', update);
  }, []);

  const charMap = new Map<string, Character>(
    (characters ?? []).map((c) => [c.id, c])
  );

  const nodeIds = (characters ?? []).map((c) => c.id);
  const links = (relationships ?? []).map((r) => ({ source: r.from_character_id, target: r.to_character_id }));
  const positions = useForceLayout(nodeIds, links, svgSize.width, svgSize.height);

  const selectedChar = selectedCharId ? charMap.get(selectedCharId) : null;
  const selectedRels = selectedCharId
    ? (relationships ?? []).filter(
        (r) => r.from_character_id === selectedCharId || r.to_character_id === selectedCharId
      )
    : [];

  const handleAddRelationship = async () => {
    if (!fromId || !toId) { setAddError('Select both characters'); return; }
    if (fromId === toId) { setAddError('Cannot relate a character to itself'); return; }
    setSubmitting(true);
    setAddError('');
    try {
      const req: CreateRelationshipRequest = {
        from_character_id: fromId,
        to_character_id: toId,
        relationship_type: relType,
        notes: relNotes.trim() || undefined,
      };
      await createRelationship(campaignId!, req);
      setShowAddModal(false);
      setFromId(''); setToId(''); setRelType('ally'); setRelNotes('');
      refetchRels();
    } catch (e: unknown) {
      setAddError(e instanceof Error ? e.message : String(e));
    } finally {
      setSubmitting(false);
    }
  };

  const handleDeleteRel = async (relId: string) => {
    try {
      await deleteRelationship(campaignId!, relId);
      refetchRels();
    } catch (e: unknown) {
      console.error('Delete relationship failed', e);
    }
  };

  if (charsLoading || relsLoading) return <div className="page"><LoadingSpinner /></div>;
  if (charsError) return <div className="page"><ErrorBanner message={charsError} /></div>;
  if (relsError) return <div className="page"><ErrorBanner message={relsError} /></div>;

  const COLORS: Record<string, string> = {
    pc: '#7c6af7',
    npc: '#4caf8f',
    monster: '#e05a5a',
  };

  return (
    <div className="page-section">
      <div className="section-header">
        <h2>Relationship Map</h2>
        <button className="btn btn-primary btn-sm" onClick={() => setShowAddModal(true)}>
          + Add Relationship
        </button>
      </div>

      <div style={{ display: 'flex', gap: 16 }}>
        <div style={{ flex: 1, border: '1px solid var(--color-border, #333)', borderRadius: 8, overflow: 'hidden' }}>
          <svg
            ref={svgRef}
            width={svgSize.width}
            height={svgSize.height}
            style={{ display: 'block', background: 'var(--color-surface, #1e1e2e)' }}
          >
            <defs>
              <marker id="arrow" markerWidth="8" markerHeight="8" refX="6" refY="3" orient="auto">
                <path d="M0,0 L0,6 L8,3 z" fill="#666" />
              </marker>
            </defs>
            {/* Edges */}
            {(relationships ?? []).map((rel) => {
              const a = positions[rel.from_character_id];
              const b = positions[rel.to_character_id];
              if (!a || !b) return null;
              const mx = (a.x + b.x) / 2;
              const my = (a.y + b.y) / 2;
              return (
                <g key={rel.id}>
                  <line
                    x1={a.x} y1={a.y} x2={b.x} y2={b.y}
                    stroke="#555" strokeWidth={2}
                    markerEnd="url(#arrow)"
                  />
                  <text x={mx} y={my - 4} textAnchor="middle" fill="#aaa" fontSize={10}>
                    {rel.relationship_type}
                  </text>
                </g>
              );
            })}
            {/* Nodes */}
            {(characters ?? []).map((char) => {
              const pos = positions[char.id];
              if (!pos) return null;
              const isSelected = char.id === selectedCharId;
              const color = COLORS[char.character_type] ?? '#888';
              return (
                <g
                  key={char.id}
                  transform={`translate(${pos.x},${pos.y})`}
                  style={{ cursor: 'pointer' }}
                  onClick={() => setSelectedCharId(char.id === selectedCharId ? null : char.id)}
                >
                  <circle
                    r={24}
                    fill={color}
                    stroke={isSelected ? '#fff' : 'transparent'}
                    strokeWidth={isSelected ? 2 : 0}
                    opacity={0.9}
                  />
                  <text textAnchor="middle" dominantBaseline="central" fill="#fff" fontSize={11} fontWeight="bold">
                    {char.name.substring(0, 3)}
                  </text>
                  <text y={34} textAnchor="middle" fill="#ccc" fontSize={10}>
                    {char.name.length > 12 ? char.name.substring(0, 12) + '…' : char.name}
                  </text>
                </g>
              );
            })}
          </svg>
        </div>

        {selectedChar && (
          <div style={{ width: 240, padding: 16, border: '1px solid var(--color-border, #333)', borderRadius: 8, background: 'var(--color-surface, #1e1e2e)' }}>
            <h3 style={{ margin: '0 0 8px' }}>{selectedChar.name}</h3>
            <p style={{ margin: '0 0 4px', fontSize: 13, color: '#aaa' }}>
              {selectedChar.character_type} {selectedChar.class ? `· ${selectedChar.class}` : ''} {selectedChar.race ? `· ${selectedChar.race}` : ''}
            </p>
            <p style={{ margin: '0 0 12px', fontSize: 13, color: '#aaa' }}>Level {selectedChar.level}</p>

            <h4 style={{ margin: '0 0 8px', fontSize: 13 }}>Relationships</h4>
            {selectedRels.length === 0 ? (
              <p style={{ fontSize: 12, color: '#666' }}>No relationships yet.</p>
            ) : (
              <ul style={{ padding: 0, margin: 0, listStyle: 'none' }}>
                {selectedRels.map((rel) => {
                  const otherId = rel.from_character_id === selectedCharId ? rel.to_character_id : rel.from_character_id;
                  const other = charMap.get(otherId);
                  const direction = rel.from_character_id === selectedCharId ? '→' : '←';
                  return (
                    <li key={rel.id} style={{ display: 'flex', alignItems: 'center', gap: 6, marginBottom: 6, fontSize: 12 }}>
                      <span style={{ flex: 1 }}>{direction} {other?.name ?? 'Unknown'}: <em>{rel.relationship_type}</em></span>
                      <button
                        className="btn btn-sm"
                        style={{ padding: '2px 6px', fontSize: 11 }}
                        onClick={() => handleDeleteRel(rel.id)}
                      >
                        ✕
                      </button>
                    </li>
                  );
                })}
              </ul>
            )}
          </div>
        )}
      </div>

      {showAddModal && (
        <Modal title="Add Relationship" onClose={() => setShowAddModal(false)}>
          {addError && <div className="form-error-banner">{addError}</div>}
          <div className="form">
            <div className="form-field">
              <label className="form-label">From</label>
              <select className="form-input" value={fromId} onChange={(e) => setFromId(e.target.value)}>
                <option value="">— select character —</option>
                {(characters ?? []).map((c) => (
                  <option key={c.id} value={c.id}>{c.name}</option>
                ))}
              </select>
            </div>
            <div className="form-field">
              <label className="form-label">To</label>
              <select className="form-input" value={toId} onChange={(e) => setToId(e.target.value)}>
                <option value="">— select character —</option>
                {(characters ?? []).map((c) => (
                  <option key={c.id} value={c.id}>{c.name}</option>
                ))}
              </select>
            </div>
            <div className="form-field">
              <label className="form-label">Type</label>
              <select className="form-input" value={relType} onChange={(e) => setRelType(e.target.value)}>
                {RELATIONSHIP_TYPES.map((t) => (
                  <option key={t} value={t}>{t}</option>
                ))}
              </select>
            </div>
            <div className="form-field">
              <label className="form-label">Notes</label>
              <textarea className="form-input" rows={3} value={relNotes} onChange={(e) => setRelNotes(e.target.value)} placeholder="Optional context…" />
            </div>
            <div className="form-actions">
              <button className="btn btn-primary" onClick={handleAddRelationship} disabled={submitting}>
                {submitting ? 'Adding…' : 'Add Relationship'}
              </button>
              <button className="btn" onClick={() => setShowAddModal(false)} disabled={submitting}>Cancel</button>
            </div>
          </div>
        </Modal>
      )}
    </div>
  );
}
