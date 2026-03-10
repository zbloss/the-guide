import { useState, useRef } from 'react';
import { NavLink } from 'react-router-dom';
import { useApi } from '../../hooks/useApi';
import { listCampaigns } from '../../api/campaigns';
import type { Campaign } from '../../api/types';

function getStoredOrder(): string[] {
  try { return JSON.parse(localStorage.getItem('campaign_order') ?? '[]'); } catch { return []; }
}

function applyOrder(campaigns: Campaign[], order: string[]): Campaign[] {
  if (!order.length) return campaigns;
  const map = new Map(campaigns.map((c) => [c.id, c]));
  const ordered = order.filter((id) => map.has(id)).map((id) => map.get(id)!);
  const rest = campaigns.filter((c) => !order.includes(c.id));
  return [...ordered, ...rest];
}

export function Sidebar() {
  const { data: rawCampaigns } = useApi<Campaign[]>(listCampaigns, []);
  const [order, setOrder] = useState<string[]>(getStoredOrder);
  const dragId = useRef<string | null>(null);

  const campaigns = rawCampaigns ? applyOrder(rawCampaigns, order) : [];

  const handleDragStart = (id: string) => { dragId.current = id; };

  const handleDragOver = (e: React.DragEvent, targetId: string) => {
    e.preventDefault();
    if (!dragId.current || dragId.current === targetId) return;
    const ids = campaigns.map((c) => c.id);
    const from = ids.indexOf(dragId.current);
    const to = ids.indexOf(targetId);
    if (from === -1 || to === -1) return;
    const next = [...ids];
    next.splice(from, 1);
    next.splice(to, 0, dragId.current);
    setOrder(next);
    localStorage.setItem('campaign_order', JSON.stringify(next));
  };

  return (
    <nav className="sidebar">
      <div className="sidebar-title">
        <span className="sidebar-icon">⚔️</span>
        <span>The Guide</span>
      </div>

      <div className="sidebar-section">
        <div className="sidebar-section-label">Campaigns</div>
        {campaigns.map((c) => (
          <div
            key={c.id}
            draggable
            onDragStart={() => handleDragStart(c.id)}
            onDragOver={(e) => handleDragOver(e, c.id)}
            className="sidebar-draggable"
          >
            <NavLink
              to={`/campaigns/${c.id}`}
              className={({ isActive }) => `sidebar-link ${isActive ? 'active' : ''}`}
            >
              ⠿ {c.name}
            </NavLink>
          </div>
        ))}
        <NavLink to="/" end className={({ isActive }) => `sidebar-link sidebar-link-new ${isActive ? 'active' : ''}`}>
          + All Campaigns
        </NavLink>
      </div>

      <div className="sidebar-bottom">
        <NavLink to="/playstyle" className={({ isActive }) => `sidebar-link ${isActive ? 'active' : ''}`}>
          🎭 Playstyle
        </NavLink>
        <NavLink to="/documents" className={({ isActive }) => `sidebar-link ${isActive ? 'active' : ''}`}>
          📚 Global Docs
        </NavLink>
        <NavLink to="/health" className={({ isActive }) => `sidebar-link ${isActive ? 'active' : ''}`}>
          🩺 Health
        </NavLink>
      </div>
    </nav>
  );
}
