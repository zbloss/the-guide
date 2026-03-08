import { useApi } from '../hooks/useApi';
import { getHealth, getVersion } from '../api/health';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import type { HealthResponse, VersionResponse } from '../api/types';

export function HealthPage() {
  const { data: health, loading: hLoading, error: hError, refetch: refetchHealth } = useApi<HealthResponse>(getHealth, []);
  const { data: version, loading: vLoading, error: vError } = useApi<VersionResponse>(getVersion, []);

  return (
    <div className="page">
      <div className="page-header">
        <h1>System Health</h1>
        <button className="btn btn-sm" onClick={refetchHealth}>Refresh</button>
      </div>

      {(hLoading || vLoading) && <LoadingSpinner />}
      {hError && <ErrorBanner message={hError} />}
      {vError && <ErrorBanner message={vError} />}

      <div className="health-grid">
        <div className="health-card">
          <h3>Backend Status</h3>
          {health ? (
            <div className={`health-status ${health.status === 'ok' ? 'health-ok' : 'health-err'}`}>
              {health.status === 'ok' ? '✓ Online' : '✗ Error'}
              <span className="health-detail">{health.status}</span>
            </div>
          ) : (
            <div className="health-status health-unknown">Checking…</div>
          )}
        </div>

        {version && (
          <div className="health-card">
            <h3>Version Info</h3>
            <table className="data-table">
              <tbody>
                <tr><td><strong>Name</strong></td><td>{version.name}</td></tr>
                <tr><td><strong>Version</strong></td><td>{version.version}</td></tr>
              </tbody>
            </table>
          </div>
        )}

        <div className="health-card">
          <h3>Configuration</h3>
          <table className="data-table">
            <tbody>
              <tr><td><strong>Backend URL</strong></td><td>http://localhost:8000</td></tr>
              <tr><td><strong>LLM Provider</strong></td><td>Ollama (local)</td></tr>
              <tr><td><strong>Database</strong></td><td>SQLite (WAL mode)</td></tr>
              <tr><td><strong>Vector Store</strong></td><td>Qdrant (optional)</td></tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
