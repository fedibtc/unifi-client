import { Wifi, Clock } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useLocation, Navigate } from 'react-router-dom';

type LocationState = {
  expiresAt: string;
};

function formatTimeRemaining(seconds_remaining: number): string {
  if (seconds_remaining < 0) return '0:00:00';
  
  const hours = Math.floor(seconds_remaining / (60 * 60));
  const minutes = Math.floor((seconds_remaining % (60 * 60)) / 60);
  const seconds = seconds_remaining % 60;
  
  return `${hours}:${minutes.toString().padStart(2, '0')}:${seconds.toString().padStart(2, '0')}`;
}

function unixTimeToLocalDate(unixTime: number) {
  const date = new Date(unixTime * 1000);
  
  return date.toLocaleString(undefined, {
    hour: 'numeric',
    minute: '2-digit',
    second: '2-digit',
  });
}

export default function SuccessPage() {
  const location = useLocation();
  const [timeRemaining, setTimeRemaining] = useState<number>(0);
  
  // Redirect if no expires_at was passed
  if (!location.state?.expiresAt) {
    return <Navigate to="/" replace />;
  }
  
  useEffect(() => {
    const expiresAt = new Date(location.state.expiresAt).getTime();
    let interval: NodeJS.Timeout;
    
    const updateTimer = () => {
      const now = Math.floor(Date.now() / 1000);
      const remaining = expiresAt - now;
      setTimeRemaining(remaining);
      
      // Stop the timer when we reach 0
      if (remaining <= 0) {
        clearInterval(interval);
      }
    };
    
    // Update immediately and then every second
    updateTimer();
    interval = setInterval(updateTimer, 1000);
    
    return () => clearInterval(interval);
  }, [location.state.expiresAt]);
  
  return (
    <div className="min-h-screen flex items-center justify-center bg-gradient-to-b from-gray-50 to-gray-100 p-4">
      <div className="w-full max-w-2xl bg-white rounded-xl shadow-lg overflow-hidden">
        <div className="p-6 sm:p-8">
          <div className="text-center">
            <div className="flex justify-center items-center gap-3 mb-4">
              <Wifi className="h-8 w-8 text-green-600" />
              <h1 className="text-3xl font-bold text-gray-900">
                Connected Successfully
              </h1>
            </div>
            <p className="mt-2 text-lg text-gray-600">
              You now have internet access
            </p>
          </div>

          <div className="mt-8 space-y-6">
            <div className="bg-green-50 rounded-lg p-6 text-center">
              <div className="flex justify-center items-center gap-2 mb-3">
                <Clock className="h-6 w-6 text-green-600" />
                <h2 className="text-xl font-semibold text-green-800">Time Remaining</h2>
              </div>
              <p className="text-4xl font-mono font-bold text-green-700">
                {formatTimeRemaining(timeRemaining)}
              </p>
            </div>
            
            <div className="text-center text-sm text-gray-500">
              Your session will expire at{' '}
              {unixTimeToLocalDate(location.state.expiresAt)}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}