// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services

const PREFIX = "ashiato-collector:1:";

export interface SetupString {
  url: string;
  token: string;
}

export function buildSetupString(url: string, token: string): string {
  const json = JSON.stringify({ url, token });
  const bytes = new TextEncoder().encode(json);
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return PREFIX + btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

export function parseSetupString(input: string): SetupString | null {
  const encoded = input.trim();
  if (!encoded.startsWith(PREFIX)) return null;
  const payload = encoded.slice(PREFIX.length);
  if (!payload || !/^[A-Za-z0-9_-]+$/.test(payload) || payload.length % 4 === 1) return null;
  try {
    const padded = payload.replace(/-/g, "+").replace(/_/g, "/") + "=".repeat((4 - (payload.length % 4)) % 4);
    const binary = atob(padded);
    const bytes = Uint8Array.from(binary, (character) => character.charCodeAt(0));
    const value: unknown = JSON.parse(new TextDecoder().decode(bytes));
    if (typeof value !== "object" || value === null) return null;
    const object = value as Record<string, unknown>;
    if (typeof object.url !== "string" || !object.url || typeof object.token !== "string" || !object.token) return null;
    return { url: object.url, token: object.token };
  } catch {
    return null;
  }
}
