import { useApi } from '../hooks/useApi';
import { listGlobalDocs, uploadGlobalDoc, getGlobalDoc, ingestGlobalDoc, deleteGlobalDoc } from '../api/documents';
import { DocumentList } from '../components/documents/DocumentList';
import { UploadForm } from '../components/documents/UploadForm';
import { IngestButton } from '../components/documents/IngestButton';
import { ConfirmButton } from '../components/common/ConfirmButton';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import type { GlobalDocument } from '../api/types';

export function GlobalDocumentsPage() {
  const { data: docs, loading, error, refetch } = useApi<GlobalDocument[]>(listGlobalDocs, []);

  const handleUpload = async (file: File) => {
    await uploadGlobalDoc(file);
    refetch();
  };

  const handleDelete = async (docId: string) => {
    await deleteGlobalDoc(docId);
    refetch();
  };

  return (
    <div className="page">
      <div className="page-header">
        <h1>Global Documents</h1>
      </div>
      <p className="page-subtitle">Upload D&D rulebooks and reference materials shared across all campaigns.</p>

      <UploadForm onUpload={handleUpload} />

      {loading && <LoadingSpinner />}
      {error && <ErrorBanner message={error} />}

      {docs && (
        <DocumentList
          documents={docs}
          renderActions={(doc) => {
            const d = doc as GlobalDocument;
            return (
              <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                <IngestButton
                  docId={d.id}
                  currentStatus={d.ingestion_status}
                  ingestionProgress={d.ingestion_progress}
                  onIngest={() => ingestGlobalDoc(d.id).then(() => {})}
                  onPoll={() => getGlobalDoc(d.id)}
                  onComplete={refetch}
                />
                <ConfirmButton
                  label="Delete"
                  confirmLabel="Delete document?"
                  variant="danger"
                  onConfirm={() => handleDelete(d.id)}
                />
              </div>
            );
          }}
        />
      )}
    </div>
  );
}
