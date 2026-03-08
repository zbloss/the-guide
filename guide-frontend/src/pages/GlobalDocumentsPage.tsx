import { useApi } from '../hooks/useApi';
import { listGlobalDocs, uploadGlobalDoc, getGlobalDoc, ingestGlobalDoc } from '../api/documents';
import { DocumentList } from '../components/documents/DocumentList';
import { UploadForm } from '../components/documents/UploadForm';
import { IngestButton } from '../components/documents/IngestButton';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import type { GlobalDocument } from '../api/types';

export function GlobalDocumentsPage() {
  const { data: docs, loading, error, refetch } = useApi<GlobalDocument[]>(listGlobalDocs, []);

  const handleUpload = async (file: File) => {
    await uploadGlobalDoc(file);
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
              <IngestButton
                docId={d.id}
                currentStatus={d.status}
                onIngest={() => ingestGlobalDoc(d.id).then(() => {})}
                onPoll={() => getGlobalDoc(d.id)}
              />
            );
          }}
        />
      )}
    </div>
  );
}
