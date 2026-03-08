import { NavLink } from 'react-router-dom';
import { useApi } from '../../hooks/useApi';
import { listCampaigns } from '../../api/campaigns';
import type { Campaign } from '../../api/types';

export function Sidebar() {
  const { data: campaigns } = useApi<Campaign[]>(listCampaigns, []);

  return (
    <nav className="sidebar">
      <div className="sidebar-title">
        <span className="sidebar-icon">⚔️</span>
        <span>The Guide</span>
      </div>

      <div className="sidebar-section">
        <div className="sidebar-section-label">Campaigns</div>
        {campaigns?.map((c) => (
          <NavLink
            key={c.id}
            to={`/campaigns/${c.id}`}
            className={({ isActive }) => `sidebar-link ${isActive ? 'active' : ''}`}
          >
            {c.name}
          </NavLink>
        ))}
        <NavLink to="/" end className={({ isActive }) => `sidebar-link sidebar-link-new ${isActive ? 'active' : ''}`}>
          + All Campaigns
        </NavLink>
      </div>

      <div className="sidebar-bottom">
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
