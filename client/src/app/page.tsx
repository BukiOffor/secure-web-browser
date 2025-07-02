'use client';

import ServerForm from "@/components/Server";
import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export default function Home() {
   useEffect(() => {
    const unlisten = listen<string>('url', (event) => {
      console.log('Received server URL:', event.payload);
      // You can route, store the value, or show a toast
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);
  return (
    <ServerForm/>
  );
}
