// SPDX-License-Identifier: AGPL-3.0-only
/** `vite.config.ts` / `vitest.config.ts` が `define` で埋める画面の組み立ての印（`web/build-stamp.ts`）。 */
declare const __ASHIATO_WEB_VERSION__: string;

/** `@types/node` は入れない（DOM 前提の src の型を動かさない）。`web/build-stamp.ts` が使う分だけ宣言する。 */
declare module "node:child_process" {
  export function execFileSync(
    file: string,
    args: string[],
    options: { encoding: "utf8"; stdio: string[]; env: Record<string, string | undefined> },
  ): string;
}
