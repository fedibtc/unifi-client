import { Wifi, WifiOff, Globe } from 'lucide-react';

export function StatusMessage({
  type, message, internetAccess
}: {
  type: 'success' | 'error' | null;
  message: string;
  internetAccess: boolean;
}) {
  if (!message) return null;

  const styles = type === 'error'
    ? 'bg-red-50 border-red-200 text-red-700'
    : 'bg-green-50 border-green-200 text-green-700';

  return (
    <div className={`mt-4 p-4 rounded-lg border ${styles}`}>
      <div className="flex items-center gap-2">
        {type === 'success' ? <Wifi className="h-4 w-4" /> : <WifiOff className="h-4 w-4" />}
        <span>{message}</span>
      </div>
      {type === 'success' && (
        <div className="mt-2 flex items-center gap-2 text-sm">
          <Globe className="h-4 w-4" />
          <span>Internet Access: {internetAccess ? 'Available' : 'Limited'}</span>
        </div>
      )}
    </div>
  );
};