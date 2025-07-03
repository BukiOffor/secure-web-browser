"use client";

import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { exit } from "@tauri-apps/plugin-process";

export default function ExitModal() {
    const [show, setShow] = useState(false);
    const [password, setPassword] = useState("");
    const [loading, setLoading] = useState(false);
    const [response, setResponse] = useState<string | null>(null);

    useEffect(() => {
        let unlisten = listen("start::exit", () => {
            setShow(true);
        });
        return () => {
            unlisten.then((fn) => fn());
        };
    }, []);

    const handleSubmit = async () => {
        setLoading(true);
        try {
            const result = await invoke<boolean>("exit_exam", { password });
            console.log(result);
            if (result) {
                setLoading(false);
                await sleep(8000);
                await exit(0); // Kill the app
            } else {
                setResponse(`Incorrect Password`);
            }
        } catch (err: any) {
            console.error(err);
            setResponse(`${err || "Error: Failed to connect."}`);
        } finally {
            setLoading(false);
        }
    };

    if (!show) return null;

    return (
        <div className="fixed inset-0 z-50 bg-black bg-opacity-50 flex items-center justify-center">
            <div className="bg-white rounded-2xl p-8 w-96 space-y-4 shadow-xl">
                <h2 className="text-xl font-semibold text-center, text-black">
                    Enter Password
                </h2>
                <input
                    type="password"
                    className="w-full px-4 py-2 border border-black text-black rounded-lg focus:outline-none focus:ring"
                    placeholder="***************"
                    value={password}
                    onChange={(e) => setPassword(e.target.value)}
                />
                <button
                    onClick={handleSubmit}
                    className="w-full flex justify-center items-center gap-2 bg-blue-600 hover:bg-blue-700 text-white py-2 rounded-lg disabled:opacity-50"
                    disabled={loading}
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
                            Submiting...
                        </>
                    ) : (
                        "Submit"
                    )}
                </button>
                {response && (
                    <p className="mt-4 text-sm text-center text-gray-600">{response}</p>
                )}
            </div>
        </div>
    );
}
function sleep(ms: number) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}
