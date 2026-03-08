import { useApi } from './useApi';
import { getCampaign } from '../api/campaigns';
import type { Campaign } from '../api/types';

export function useCampaign(campaignId: string | undefined): {
  campaign: Campaign | null;
  loading: boolean;
  error: string | null;
  refetch: () => void;
} {
  const { data, loading, error, refetch } = useApi<Campaign>(
    () => campaignId ? getCampaign(campaignId) : Promise.reject(new Error('No campaign ID')),
    [campaignId],
  );
  return { campaign: data, loading, error, refetch };
}
