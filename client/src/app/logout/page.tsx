'use client';

import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

export default function Logout() {
  const [password, setPassword] = useState('');
  const [loading, setLoading] = useState(false);
  const [response, setResponse] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setResponse(null);

    try {
      const result = await invoke<string>('set_server', {
        url: password,
      });

      setResponse(`Success: waiting for redirect ...`);
    } catch (error: any) {
      console.log("Error: ", error)
      setResponse(`Error: ${error.message || 'Failed to connect.'}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-gradient-to-br from-gray-100 to-gray-200 p-4">
      <form
        onSubmit={handleSubmit}
        className="bg-white shadow-lg rounded-lg p-8 w-full max-w-md"
      >
        <h1 className="text-2xl font-semibold mb-4 text-black">
          End Examination
        </h1>

        <label htmlFor="serverUrl" className="block text-sm font-medium text-black mb-2">
          Enter Password
        </label>
        <input
          type="password"
          id="serverUrl"
          name="serverUrl"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          className="w-full px-4 py-2 border border-gray-300 rounded-md shadow-sm focus:ring-blue-500 focus:border-blue-500 text-black"
          placeholder="********"
          required
        />

        <button
          type="submit"
          disabled={loading}
          className="mt-4 w-full bg-blue-600 hover:bg-blue-700 text-white font-medium py-2 px-4 rounded-md transition"
        >
          {loading ? 'Connecting...' : 'Submit'}
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
