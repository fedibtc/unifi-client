import { useEffect, useState } from 'react';
import { Wifi, Loader2 } from 'lucide-react';
import { redirect, useNavigate, useSearchParams } from 'react-router';

import type { Route } from './+types/portal';
import { StatusMessage } from '~/components/status-message';
import { DurationSelector } from '~/components/duration-selection';

type GuestAuthRequest = {
  client_mac_address: string | null;
  access_point_mac_address: string | null;
  captive_portal_timestamp: number | null;
  requested_url: string | null;
  wifi_network: string | null;
  duration_minutes: number | null;
  data_quota_megabytes: number | null;
};

const durationOptions = [
  { value: 5, label: '5 Minutes', description: 'Quick access' },
  { value: 10, label: '10 Minutes', description: 'Short session' },
  { value: 30, label: '30 Minutes', description: 'Regular session' },
  { value: 60, label: '1 Hour', description: 'Extended access' },
  { value: 720, label: '12 Hours', description: 'Half day access' },
  { value: 1440, label: '24 Hours', description: 'Full day access' },
];

export function meta({ }: Route.MetaArgs) {
  return [
    { title: "UniFi Cafe" },
    { name: "description", content: "Community-Powered Internet Access" },
  ];
}

export default function PortalPage() {
  const navigate = useNavigate();
  const [searchParams] = useSearchParams()
  const [status, setStatus] = useState({
    loading: false,
    type: null as 'success' | 'error' | null,
    message: '',
    internetAccess: false,
  });
  const [selectedDuration, setSelectedDuration] = useState(10);

  const [guestAuthRequest, setGuestAuthRequest] = useState<GuestAuthRequest>({
    client_mac_address: searchParams.get('id'),
    access_point_mac_address: searchParams.get('ap'),
    captive_portal_timestamp: searchParams.get('t') ? parseInt(searchParams.get('t')!) : null,
    requested_url: searchParams.get('url'),
    wifi_network: searchParams.get('ssid'),
    duration_minutes: null,
    data_quota_megabytes: null,
  });

  // ONLY FOR DEBUGGING WHEN TESTING WITHOUT A UNIFI CONTROLLER
  // If the captive_portal_timestamp is not set, set it to the current time
  guestAuthRequest.captive_portal_timestamp ??= Math.floor(Date.now() / 1000);

  useEffect(() => {
    if (!guestAuthRequest.client_mac_address) {
      setStatus({
        loading: false,
        type: 'error',
        message: 'No client MAC address found in URL. Please check your connection.',
        internetAccess: false,
      });
      console.log('No client MAC found in URL parameters');
    }

    setGuestAuthRequest(prev => ({
      ...prev,
      duration_minutes: selectedDuration,
    }));
  }, [selectedDuration]);

  const authenticateUser = async () => {
    if (!guestAuthRequest.client_mac_address) {
      setStatus({
        loading: false,
        type: 'error',
        message: 'Client MAC address not found. Please check your connection.',
        internetAccess: false,
      });
      return;
    }

    setStatus({
      loading: true,
      type: null,
      message: 'Connecting to network...',
      internetAccess: false,
    });

    try {
      console.log('Authenticating with duration:', selectedDuration, 'minutes');

      console.log('guestAuthRequest', guestAuthRequest);

      const response = await fetch('http://localhost:8080/guests/authorize', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          ...guestAuthRequest,
          duration_minutes: selectedDuration,
        })
      });

      const data = await response.json();
      console.log('Authentication response:', data);

      if (response.ok) {
        setStatus({
          loading: false,
          type: 'success',
          message: `Connected successfully!`,
          internetAccess: true,
        });

        // Wait 2 seconds before redirecting to show the success message
        setTimeout(() => {
          navigate('/success', {
            state: { expiresAt: data.expires_at },
            replace: true
          });
        }, 2000);
      } else {
        setStatus({
          loading: false,
          type: 'error',
          message: data.message || 'Authentication failed. Please try again.',
          internetAccess: false,
        });
      }
    } catch (error) {
      console.error('Authentication error:', error);
      setStatus({
        loading: false,
        type: 'error',
        message: 'Unable to connect to the server. Please check your connection.',
        internetAccess: false,
      });
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-gradient-to-b from-gray-50 to-gray-100 p-4">
      <div className="w-full max-w-2xl bg-white rounded-xl shadow-lg overflow-hidden">
        <div className="p-6 sm:p-8">
          <div className="text-center">
            <h1 className="text-3xl font-bold text-gray-900">
              UniFi Cafe Portal
            </h1>
            <p className="mt-2 text-lg text-gray-600">
              Select your preferred duration and connect to the network
            </p>
          </div>

          <div className="mt-8 space-y-6">
            <div className="space-y-4">
              <h3 className="text-lg font-semibold text-gray-900">Access Duration</h3>
              <DurationSelector
                selected={selectedDuration}
                onChange={setSelectedDuration}
                disabled={status.loading}
                durationOptions={durationOptions}
              />
            </div>

            <button
              onClick={authenticateUser}
              disabled={status.loading}
              className={`
                w-full h-12 flex items-center justify-center gap-2 
                rounded-lg text-white text-lg font-medium
                ${status.loading
                  ? 'bg-blue-400 cursor-not-allowed'
                  : 'bg-blue-600 hover:bg-blue-700'}
                transition-colors duration-200
              `}
            >
              {status.loading ? (
                <>
                  <Loader2 className="h-5 w-5 animate-spin" />
                  <span>Authenticating...</span>
                </>
              ) : (
                <>
                  <Wifi className="h-5 w-5" />
                  <span>Connect for {selectedDuration} minutes</span>
                </>
              )}
            </button>

            <StatusMessage
              type={status.type}
              message={status.message}
              internetAccess={status.internetAccess}
            />
          </div>
        </div>
      </div>
    </div>
  )
}