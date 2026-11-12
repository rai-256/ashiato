// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
// `@types/node` は入れない（playwright.config.ts の注記）。playwright.config.ts が使う分だけを宣言する。
declare module "node:child_process" {
  export function execFileSync(file: string, args: readonly string[], options: { encoding: "utf8" }): string;
}

declare module "node:fs" {
  export function mkdirSync(path: string, options?: { recursive?: boolean }): void;
  export function rmSync(path: string, options?: { recursive?: boolean; force?: boolean }): void;
  export function writeFileSync(path: string, data: string): void;
  export function readFileSync(path: string, encoding: "utf8"): string;
}
