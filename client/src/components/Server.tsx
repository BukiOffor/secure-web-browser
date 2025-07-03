'use client';

import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export default function ServerForm() {
  const [serverUrl, setServerUrl] = useState('');
  const [loading, setLoading] = useState(false);
  const [response, setResponse] = useState<string | null>(null);


   useEffect(() => {
    const fetchUrl = async () => {
      try {
        const result = await invoke<any>('server_url');
        if (result != null) {
          setServerUrl(result);
        }
        //console.log('Fetched URL from Tauri:', result);
      } catch (error) {
        console.error('Failed to fetch URL from Tauri:', error);
      }
    };

    fetchUrl(); 
  }, [serverUrl]);





  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setResponse(null);

    try {
      const result = await invoke<string>('set_server', {
        url: serverUrl,
      });

      setResponse(`Success: waiting for redirect ...`);  
    } catch (error: any) {
      console.log("Error: ", error)
      setResponse(`Error: ${error.message || 'Failed to connect.'}`);
      setLoading(false);
    } finally {
      //setLoading(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-gradient-to-br from-gray-100 to-gray-200 p-4">
      <form
        onSubmit={handleSubmit}
        className="bg-white shadow-lg rounded-lg p-8 w-full max-w-md"
      >
        <h1 className="text-2xl font-semibold mb-4 text-black">
          Connect to Server
        </h1>

        <label htmlFor="serverUrl" className="block text-sm font-medium text-black mb-2">
          Server URL
        </label>
        <input
          type="text"
          id="serverUrl"
          name="serverUrl"
          value={serverUrl}
          onChange={(e) => setServerUrl(e.target.value)}
          className="w-full px-4 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 text-black"
          placeholder="https://example.com"
          required
        />

       <button
  type="submit"
  disabled={loading}
  className="mt-4 w-full bg-blue-600 hover:bg-blue-700 text-white font-medium py-2 px-4 rounded-md transition flex items-center justify-center"
>
  {loading ? (
    <>
      <svg
        className="animate-spin h-5 w-5 mr-2 text-white"
        xmlns="http://www.w3.org/2000/svg"
        fill="none"
        viewBox="0 0 24 24"
      >
        <circle
          className="opacity-25"
          cx="12"
          cy="12"
          r="10"
          stroke="currentColor"
          strokeWidth="4"
        />
        <path
          className="opacity-75"
          fill="currentColor"
          d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"
        />
      </svg>
      Connecting...
    </>
  ) : (
    'Submit'
  )}
</button>


        {response && (
          <p className="mt-4 text-sm text-center text-gray-600">
            {response}
          </p>
        )}
      </form>
    </div>
  );
}
