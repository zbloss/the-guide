import { useState, useCallback } from 'react';
import { useApi } from '../hooks/useApi';
import { listCampaigns, deleteCampaign } from '../api/campaigns';
import { listCampaignDocs, listGlobalDocs, deleteCampaignDoc, deleteGlobalDoc } from '../api/documents';
import { ConfirmButton } from '../components/common/ConfirmButton';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import type { Campaign, CampaignDocument, GlobalDocument } from '../api/types';

function CampaignDocRow({
  campaignId,
  doc,
  onDeleted,
}: {
  campaignId: string;
  doc: CampaignDocument;
  onDeleted: () => void;
}) {
  const handleDelete = useCallback(async () => {
    await deleteCampaignDoc(campaignId, doc.id);
    onDeleted();
  }, [campaignId, doc.id, onDeleted]);

  return (
    <tr>
      <td>{doc.filename}</td>
      <td>{doc.ingestion_status}</td>
      <td>{doc.document_kind}</td>
      <td>
        <ConfirmButton label="Delete" confirmLabel="Delete document?" variant="danger" onConfirm={handleDelete} />
      </td>
    </tr>
  );
}

function CampaignDocsSection({ campaignId }: { campaignId: string }) {
  const fetcher = useCallback(() => listCampaignDocs(campaignId), [campaignId]);
  const { data: docs, loading, error, refetch } = useApi<CampaignDocument[]>(fetcher, [campaignId]);

  if (loading) return <p className="text-muted" style={{ padding: '0.5rem 1rem' }}>Loading docs…</p>;
  if (error) return <p className="text-danger" style={{ padding: '0.5rem 1rem' }}>{error}</p>;
  if (!docs || docs.length === 0) return <p className="text-muted" style={{ padding: '0.5rem 1rem' }}>No documents.</p>;

  return (
    <table className="table table-sm" style={{ marginLeft: '2rem' }}>
      <thead>
        <tr>
          <th>Filename</th>
          <th>Status</th>
          <th>Kind</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {docs.map((doc) => (
          <CampaignDocRow key={doc.id} campaignId={campaignId} doc={doc} onDeleted={refetch} />
        ))}
      </tbody>
    </table>
  );
}

function CampaignRow({
  campaign,
  onDeleted,
}: {
  campaign: Campaign;
  onDeleted: () => void;
}) {
  const [expanded, setExpanded] = useState(false);

  const handleDelete = useCallback(async () => {
    await deleteCampaign(campaign.id);
    onDeleted();
  }, [campaign.id, onDeleted]);

  return (
    <>
      <tr>
        <td>
          <button
            className="btn btn-sm"
            onClick={() => setExpanded((v) => !v)}
            style={{ marginRight: '0.5rem' }}
          >
            {expanded ? '▾' : '▸'}
          </button>
          {campaign.name}
        </td>
        <td>{campaign.game_system ?? '—'}</td>
        <td>
          <ConfirmButton label="Delete" confirmLabel="Delete campaign?" variant="danger" onConfirm={handleDelete} />
        </td>
      </tr>
      {expanded && (
        <tr>
          <td colSpan={3} style={{ padding: 0 }}>
            <CampaignDocsSection campaignId={campaign.id} />
          </td>
        </tr>
      )}
    </>
  );
}

function GlobalDocRow({
  doc,
  onDeleted,
}: {
  doc: GlobalDocument;
  onDeleted: () => void;
}) {
  const handleDelete = useCallback(async () => {
    await deleteGlobalDoc(doc.id);
    onDeleted();
  }, [doc.id, onDeleted]);

  return (
    <tr>
      <td>{doc.title}</td>
      <td>{doc.filename}</td>
      <td>{doc.ingestion_status}</td>
      <td>
        <ConfirmButton label="Delete" confirmLabel="Delete global doc?" variant="danger" onConfirm={handleDelete} />
      </td>
    </tr>
  );
}

export function AdminPage() {
  const { data: campaigns, loading: campsLoading, error: campsError, refetch: refetchCamps } = useApi<Campaign[]>(listCampaigns, []);
  const { data: globalDocs, loading: docsLoading, error: docsError, refetch: refetchDocs } = useApi<GlobalDocument[]>(listGlobalDocs, []);

  return (
    <div className="page-content" style={{ padding: '2rem' }}>
      <h1>Admin</h1>

      <section style={{ marginBottom: '3rem' }}>
        <h2>Campaigns</h2>
        {campsError && <ErrorBanner message={campsError} />}
        {campsLoading && <LoadingSpinner />}
        {campaigns && (
          <table className="table">
            <thead>
              <tr>
                <th>Name</th>
                <th>System</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {campaigns.map((c) => (
                <CampaignRow key={c.id} campaign={c} onDeleted={refetchCamps} />
              ))}
            </tbody>
          </table>
        )}
        {campaigns && campaigns.length === 0 && <p className="text-muted">No campaigns.</p>}
      </section>

      <section>
        <h2>Global Documents</h2>
        {docsError && <ErrorBanner message={docsError} />}
        {docsLoading && <LoadingSpinner />}
        {globalDocs && (
          <table className="table">
            <thead>
              <tr>
                <th>Title</th>
                <th>Filename</th>
                <th>Status</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {globalDocs.map((doc) => (
                <GlobalDocRow key={doc.id} doc={doc} onDeleted={refetchDocs} />
              ))}
            </tbody>
          </table>
        )}
        {globalDocs && globalDocs.length === 0 && <p className="text-muted">No global documents.</p>}
      </section>
    </div>
  );
}
