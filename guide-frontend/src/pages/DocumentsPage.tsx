import { useParams } from 'react-router-dom';
import { useApi } from '../hooks/useApi';
import { listCampaignDocs, uploadCampaignDoc, getCampaignDoc, ingestCampaignDoc } from '../api/documents';
import { DocumentList } from '../components/documents/DocumentList';
import { UploadForm } from '../components/documents/UploadForm';
import { IngestButton } from '../components/documents/IngestButton';
import { LoadingSpinner } from '../components/common/LoadingSpinner';
import { ErrorBanner } from '../components/common/ErrorBanner';
import type { CampaignDocument } from '../api/types';

export function DocumentsPage() {
  const { campaignId } = useParams<{ campaignId: string }>();
  const { data: docs, loading, error, refetch } = useApi<CampaignDocument[]>(
    () => listCampaignDocs(campaignId!),
    [campaignId],
  );

  const handleUpload = async (file: File) => {
    await uploadCampaignDoc(campaignId!, file);
    refetch();
  };

  return (
    <div className="page-section">
      <div className="section-header">
        <h2>Campaign Documents</h2>
      </div>

      <UploadForm onUpload={handleUpload} />

      {loading && <LoadingSpinner />}
      {error && <ErrorBanner message={error} />}

      {docs && (
        <DocumentList
          documents={docs}
          renderActions={(doc) => {
            const d = doc as CampaignDocument;
            return (
              <IngestButton
                docId={d.id}
                currentStatus={d.status}
                onIngest={() => ingestCampaignDoc(campaignId!, d.id).then(() => {})}
                onPoll={() => getCampaignDoc(campaignId!, d.id)}
              />
            );
          }}
        />
      )}
    </div>
  );
}
