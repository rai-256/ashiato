# ST13 ソースの取り方と取得の間隔を登録簿で持ち、定期取得の枠を置く（下流・実装）

change: `openspec/changes/st13-scheduled-account-fetch/`。capability `collection-coverage` / `external-ingestion` / `device-collection`。

## 深掘りに積んだ未決（A）

**なし**（`deep.md` の未回答 0。Q1〜Q6 は本人が答え、推奨と違ったのは Q3 の画面の置き場だけ）。下流で人間に返した問いは無い。

## 仮決め（B）と Ruling

design の（仮）の決定は D2・D6・D9〜D14・D16〜D23（反転条件は design.md の各節）。下流のレビューから足したもの:

- **D20（仮）** 読み出せなかった書庫（`file_unreadable`）は既読に数えず、置き場に残す。台帳の鍵は `file_unreadable:<中身の sha>`、中身が取れなければ `file_unreadable-path:<パス＋大きさ＋更新時刻の sha>`。ハッシュの失敗は 2 回の安定の後にだけ台帳に残す（final R1 / R2・R17 / R27 / R28）
- **D21（仮）** 通知が鳴るちょうどの時刻も「あと 1 日」（final R9）
- **D22（仮）** 取り方が定期取得で契機が間隔ごとでない行は、CHECK で断らず `scheduled_no_interval` で名前をログに出して取りに行かない（code_verify R26）
- **D23（仮）** 休止の知らせは最後に**届いた**（`received_at`）生存信号から読む（code_verify R24）
- **D12（仮）の書き換え** 除外の案内の印は、出したときでなく本人が答えたときに立てる（/code-review R33）

別の席による（仮）の確かめ（`provisional-review.md`）は、グラフの `provisional_review` が書く（D20〜D23 は未）。

Ruling（全 Task に `progress.md` は無い。Ruling は final の fix report `.superpowers/sdd/st13-final-fix-report.md` の全部）:

- R1 / R2 の `file_unreadable` は既読に数えず置き場に残し、台帳の鍵を中身の sha と決して一致しない形にする — 台帳は追記のみで一意索引 (user, sha, ver, outcome) のため、中身の sha のままだと後の broken_zip などの行が ON CONFLICT で落ちる — 間違っていたときの費用: 台帳の file_unreadable の行の鍵の読み替え（行は残る）。design D20（仮）
- R9 は過ぎるまで残り日数を 1 以上にする（「過ぎた」の境目は途切れの判定と同じ `>` のまま）— spec の「過ぎた」の時刻の一致を優先 — 費用: 文言 1 つ。design D21（仮）
- R13 の F3（縦串が visit しか見ない）は指示の範囲外なので直さない（各ソースの中身は `archive_flow_tests.rs` が持つ）
- web の lint の既存の 5 件（sources.ts の「 — 」の直書き）は CI を落とすので messages.ts の `note` へ移した
- R34（見回りが 1 秒ごとに登録簿を読む）は直さない — design D4 の「縮めた間隔が 1 秒以内に効く」（spec「変わってから 1 秒以内」）の帰結で、読むのは loopback の主キー 1 行 — 費用: 1 日 86400 回の小さな照会
- R36 は既読の判定の重複だけ直し、間隔の読みは 2 つのまま — 書庫の行は取り方が `server` で定期取得の条件と違い、下限の違いは DB の制約 `BETWEEN 1 AND expected_gap_sec` で実害が無い — 費用: 制約を緩める日に 2 か所を直す
- R22 / R23 の残り日数の e2e は `/api/sources` の 1 行を固定の値に差し替えて画面の文字を見る（53 日 → あと 22 日・74 日 23 時間 → あと 1 日の数は API の試験が固定）— 費用: 画面の e2e は seed の書庫から日数を数える経路を通らない。**計画からの逸脱**: tasks 9.2 の本文は「53 日前の書庫を seed に置く」のまま（凍結の tasks は書き換えない）。R16 の退役・not_exempt の e2e も同じく応答の差し替え（`web/e2e/coverage-sources.spec.ts` のコメントに理由）
- R30 は design D5 のまま定期取得を取り込み器と同じ利用者（`ASHIATO_ARCHIVE_USER_ID`）に結ぶ。起こさないときは `scheduled_disabled` を出す — 費用: 最初の取る手を足す Story が利用者の設定を分ける
- R26 は CHECK でなくログ（D22 仮）/ R24 は届いた順（D23 仮）/ R33 は答えたときに印（D12 仮）

parked: なし。

## Task

10 Task・13 項目すべて `[x]`（グラフの `mark`）。残りは spec の人間の確認待ち 1 件（「数か月操作しなくても権限が剥がれない」—— 3 か月の休止はエミュレータで早送りできない。design D12 の「物理」）。

## 通した検証（`evidence.jsonl` の最後の実行と、finish の fix の後）

| 項目 | コマンド | 終了コード |
|---|---|---|
| 1.1 | `tools/check-migrations.sh` / `tools/check-immutable.sh` | 0 / 0 |
| 1.1 | `scripts/quiet-run unit -- cargo test -p ashiato-server` | 0（Task 1 の head `b90f6220`、lib 1219 passed）／ 0（finish の fix の後 `342895ba`、新しい DB・`log_lock_waits = on`、lib 1268 + server_startup 9 passed、デッドロック・ロック待ち 0。R25） |
| 1.1 / 7.1 | `scripts/quiet-run smoke -- tools/smoke.sh` | 0 |
| 1.2 | `bash tools/check-source-intervals.sh` | 0（定数をわざと変えた一時コピーの木では rc=1: `FIX_INTERVAL_MS` 120_000 → `NG c01-location: 登録簿 60 秒 != 収集側 120 秒`、`HISTORY_INTERVAL` 7 時間 → c02-browser-history も NG。code_verify も 4 通りで rc=1 を再現） |
| 2.1 / 7.1 / 9.2 | `bash tools/check-shared-appends.sh` | 0 |
| 3.1 | `tools/check-scheduled-clock.sh` | 0（finish で入口を止める形に。壁時計を読む 7 通りを足した複製はすべて rc=1） |
| 5.1 / 9.1 | `cd web && npx vitest run` / `npx tsc --noEmit` | 0（479 passed）/ 0 |
| 5.2 / 9.2 | `cd web && STACK_RESET=1 npm run test:e2e -- coverage-sources` | 0（17 passed。finish の fix の後） |
| 9.2 | `cd web && npm run test:e2e -- archive-layout coverage-` | 0 |
| 8.1 | `tools/check-openapi.sh` | 0 |
| 10.1 | `gradlew :app:testDebugUnitTest --tests '*UnusedAppRestrictions*' --tests '*Heartbeat*'` | 0（11 本 + 29 本。finish の fix の後） |

finish の fix（`342895ba`）は、壊した木で新しい試験が落ちることを 1 件ずつ確かめた（`review/code.md` の各 R の処置と `.superpowers/sdd/st13-final-fix-report.md`）。`cargo fmt --check` / `cargo clippy --all-targets -D warnings` は 0。

## レビュー

`review/code.md`: final review（R1〜R16）→ scoped re-review（R17）→ code_verify（R18〜R26）→ security review（指摘なし）→ /code-review（R27〜R36）→ scoped re-review（ce3379e..342895b: All findings addressed、Minor 3 件は直さない）。rejected は R14・R16・R34 と R36 の一部（理由は各処置）。

## 未処置の申し送り

- `docs/handoff/ST13.md` の R28・R32・R35・M1〜M6・M9・M10 は design D16（仮）のとおり残す（R25・R34・M7・M8 は Task 4〜6 で直した）
- 定期取得の利用者を取り込み器の `ASHIATO_ARCHIVE_USER_ID` から分けるのは、最初の取る手を足す Story（R30）
