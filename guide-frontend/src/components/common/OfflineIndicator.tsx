import { useNetworkStatus } from '../../hooks/useNetworkStatus';

export function OfflineIndicator() {
  const { isOnline } = useNetworkStatus();
  if (isOnline) return null;
  return (
    <div className="offline-banner">
      You are offline. Some data may be stale. Changes will sync when you reconnect.
    </div>
  );
}
