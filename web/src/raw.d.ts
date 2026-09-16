// SPDX-License-Identifier: AGPL-3.0-only
/**
 * Vite の `?raw`（ファイルを文字列として読む）。**検査が自分のソースを読むために使う** ——
 * `master-view-limits.test.tsx` が「色の直書きが無い」を見る（NFR / design D9）。
 * `@types/node` を入れずに済ませるため、宣言だけここに置く。
 */
declare module "*?raw" {
  const content: string;
  export default content;
}
