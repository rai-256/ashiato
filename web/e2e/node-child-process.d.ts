// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
// `@types/node` は入れない（playwright.config.ts の注記）。playwright.config.ts が使う分だけを宣言する。
declare module "node:child_process" {
  export function execFileSync(file: string, args: readonly string[], options: { encoding: "utf8" }): string;
}
