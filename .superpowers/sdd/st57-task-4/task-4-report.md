# Task 4 実装報告

## 実装内容

- `/api/update-status` の型検証、状態から帯の文を作る処理、日付・理由の表示を `web/src/update.ts` に追加。
- `UpdateBand` を Root の全画面に配置し、`available` / `stale` / `available_stale` のみを非操作の `role="status"` 帯として表示。
- `SettingsView` に「更新の確認」節を追加し、有効・無効・未稼働・読めない・保留・最新版・新しい版ありの状態と版・日時を表示。
- updater の試験用ディレクトリを Playwright 起動時に渡し、各 e2e の前後に内容を空にする共有 fixture を追加。
- 14 本の本物の Chromium e2e Scenario と、状態・文面の Vitest を追加。

## テスト結果

- `scripts/verify-run 4.1`: PASS（e2e 14/14、TypeScript + Vitest 55 files / 519 tests）
- `npm run lint`: PASS
- `npx tsc --noEmit`: PASS
- `git diff --check`: PASS

## TDD Evidence

- RED: `npm run test -- --run src/__tests__/update.test.ts` を実装前に起動しようとしたが、依存導入前のため `sh: 1: vitest: not found` で停止した。これはテスト失敗ではなく環境前提の失敗だった。
- GREEN: 依存導入後、`npm run test -- --run src/__tests__/update.test.ts` は 2/2 通過。最終的に `scripts/verify-run 4.1` でも全 Vitest と e2e が通過。

## 変更ファイル

- `web/src/update.ts`
- `web/src/UpdateBand.tsx`
- `web/src/Root.tsx`
- `web/src/SettingsView.tsx`
- `web/src/messages.ts`
- `web/src/__tests__/update.test.ts`
- `web/e2e/st57-update-notice.spec.ts`
- `web/e2e/global-setup.ts`
- `web/e2e/node-child-process.d.ts`
- `web/playwright.config.ts`
- `openspec/changes/st57-update-notice/evidence.jsonl`

## 自己レビュー・懸念

- verify-run は PASS したが、ハーネスの一時作業ディレクトリに `node-compile-cache` / `playwright-transform-cache` が残った注意が出た。試験が作る updater ディレクトリは各試験後に削除している。
- 実装前の機能的な RED 出力は依存未導入に遮られたため、TDD の証跡は環境エラーを含む。
