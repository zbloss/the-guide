import { useParams } from 'react-router-dom';
import { useApi } from '../hooks/useApi';
import { getCampaignAnalytics } from '../api/campaigns';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import type { CampaignAnalytics } from '../api/types';

function StatCard({ label, value }: { label: string; value: number }) {
  return (
    <div className="stat-card">
      <div className="stat-value">{value}</div>
      <div className="stat-label">{label}</div>
    </div>
  );
}

function BarChart({ data, labelKey, valueKey }: { data: Record<string, unknown>[]; labelKey: string; valueKey: string }) {
  const max = Math.max(...data.map((d) => d[valueKey] as number), 1);
  return (
    <div className="bar-chart">
      {data.map((d, i) => {
        const val = d[valueKey] as number;
        const pct = (val / max) * 100;
        return (
          <div key={i} className="bar-row">
            <span className="bar-label">{d[labelKey] as string}</span>
            <div className="bar-track">
              <div className="bar-fill" style={{ width: `${pct}%` }} />
            </div>
            <span className="bar-value">{val}</span>
          </div>
        );
      })}
    </div>
  );
}

export function AnalyticsPage() {
  const { campaignId } = useParams<{ campaignId: string }>();
  const { data, loading, error } = useApi<CampaignAnalytics>(
    () => getCampaignAnalytics(campaignId!),
    [campaignId],
  );

  if (loading) return <div className="page-section"><LoadingSpinner /></div>;
  if (error) return <div className="page-section"><ErrorBanner message={error} /></div>;
  if (!data) return null;

  return (
    <div className="page-section">
      <div className="section-header">
        <h2>Campaign Analytics</h2>
      </div>

      <div className="stat-cards">
        <StatCard label="Sessions" value={data.sessions_count} />
        <StatCard label="Encounters" value={data.encounters_count} />
        <StatCard label="Characters" value={data.characters_count} />
      </div>

      {data.sessions_by_month.length > 0 && (
        <div className="analytics-section">
          <h3>Sessions by Month</h3>
          <BarChart data={data.sessions_by_month as unknown as Record<string, unknown>[]} labelKey="month" valueKey="count" />
        </div>
      )}

      {data.encounter_difficulty.length > 0 && (
        <div className="analytics-section">
          <h3>Encounter Difficulty</h3>
          <BarChart data={data.encounter_difficulty as unknown as Record<string, unknown>[]} labelKey="difficulty" valueKey="count" />
        </div>
      )}

      {data.sessions_by_month.length === 0 && data.encounter_difficulty.length === 0 && (
        <p className="text-muted" style={{ marginTop: 16 }}>
          No data yet. Run some sessions and encounters to see analytics.
        </p>
      )}
    </div>
  );
}
