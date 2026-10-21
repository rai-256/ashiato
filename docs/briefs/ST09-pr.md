# ST09 — 写真と動画をメタごと取り込み、写真は原本も置く

change: `openspec/changes/st09-media-capture/`（tasks 11 Task・28 項目。レビューの処置は `review/code.md`）

## 本人に戻したもの（A）

なし。積んでいた **Q18（premise）: 計測テスト（AT）の本数の数え方が常に 0 本になる**（`review/code.md` R14）は第 3 回の深掘りで回答済み（2026-10-21）。
答えどおりに `tasks.md` の Global Constraints の `AT <クラス>` を、結果の XML のファイル名でなく `androidTest-results-stage*/TEST-*.xml` の中の
`<testcase classname="….<クラス>">` の数とする定義に直し、`tools/android-emulator.sh` が段ごとの XML を `androidTest-results-stage<N>` に残すようにした
（これまで 2 段目の XML は 3 段目に上書きされていた）。前置きの定義は `plan_fix.py` の対象外なので、本人の答えを出所に直接直した。

## 仮決め（B / P）と Ruling

design の（仮）（反転条件は design.md）: D3 / D5 / D6 / D7 / D8 / D9 / D12 / D13 / D18 / D22 / D23 / D24、
**finish で足した P（失わない側の仮。`provisional.jsonl`）**:
- **D25（仮）** 受け取り済みかは、行に加えて置いたファイルが残っているかで答える（R15）。本人が D-02 から動かした・消した原本を、端末にあるうちに置き直す。
  名前の候補は記録の今の版と前の版の全部から組む（名前が変わっても 2 つ目を置かない）。反転条件: 本人が D-02 の中を手で並べ替える使い方をすると分かったとき
- **D26（仮）** D-02 のフォルダは 0700・原本は 0600（Unix。R19）。反転条件: 本人が D-02 を別のアカウントと共有して読ませたいと言ったとき

ledger の Ruling（全 Task の `Ruling:` 行の写し）:
- Task 1: D15 の 4 の「SELECT / INSERT だけ」は付与でなく書き換え禁止の門（トリガ）が担うと読む — grants.sql は全表一律で、絞ると app_role_tests の例外が要る — 間違っていたら grants.sql に 3 表の REVOKE UPDATE, DELETE を足す（移行は要らない）
- Task 5: 受理の知らせを受ける側が同じ例外を投げ続ける記録は、未送信の上限（Retention）が捨てるまで残して送り直す — 取り除くと原本の行が awaiting のまま固まる（R23）。失わない側 — 間違っていたら 1 件ぶんの無駄な送信が毎回続く（scoped re-review Minor 4。parked）
- Task 5: 破棄の後に消せなかった原本の行（R25）の消し直しはメモリにだけ覚える — 台帳に書けないときに別の台帳へ書いても同じく書けない — 間違っていたら、その間にプロセスが再起動すると行が awaiting のまま残る（送られず、欠けは報告済み。台帳の数百バイトが残るだけ）（parked）
- Task 6: 受け取り済みの確かめ（D25 仮）は大きさの一致だけで見る（同じ大きさの別の中身が残った名前にあると置き直さない）— 問い合わせのたびに最大 1 GiB を読まない — 間違っていたら、本人が手で置き替えた名前の原本が置き直されず、ST10 の突き合わせで不一致として見つかる（scoped re-review Minor 2。parked）

## Task

**11 Task・28 項目を全部埋めた。** 残した Task は無い。人間の確認に残るのは本物のカメラで撮る 2 本（`物理: device`）だけ。
AT（計測テスト）は CI の `android-instrumented` でも走る（数え方は上の Q18 の答えで直した定義）。

## レビューの処置（`review/code.md`。`review_triage.py` rc=0）

- final review（R1〜R14）: fixed 12・rejected 1（R12）・escalated 1（R14 → Q18。回答済みで直した）
- code-verify（R15〜R18）・security review（R19）・`/code-review`（R20〜R29）: fixed 14（うち P の仮 2: R15 → D25 / R19 → D26、B の仮 1: R29 → D7）・rejected 1（R28: 移行はサーバの起動では当たらず、借り先のあるソースは自分の開始日を使わない）。
  R22 は筋書きが再現しなかったが、頭 8 文字の重なり・手で置かれたファイルで永久に 422 になる道を全ハッシュの名前の候補で潰した
- scoped re-review（0f58c591）: fix が持ち込んだ Important 1（記録の名前が変わると原本が 2 つ目として置かれる）を直し、Minor 1 を直し、Minor 2 件と R25 の穴を Ruling で park

## 通した検証（終了コード）

evidence.jsonl（Task gate の証跡）: 1.1〜11.1 の全項目が PASS・rc=0（最後の全体の走行は 22bd6503。`cargo test -p ashiato-server` 1276 本）。
その後の HEAD での走行:
- `cargo test -p ashiato-server`（全体）: 1279 passed / 0 failed・rc=0（7ec57aad。code-verify。作り直した DB）
- `scripts/quiet-run instr -- tools/android-emulator.sh`: rc=0・934 秒（71f203b0。final review R2）
- Q18 の直しの後（このブランチの HEAD）: `scripts/quiet-run instr -- tools/android-emulator.sh` rc=0・978 秒。新しい `AT` の定義で数えて
  1 段目 17 本・2 段目 2 本・3 段目 3 本・失敗 0（`MediaCaptureInstrumentedTest` 2 本・`PermissionDeniedInstrumentedTest` 1 本・`MediaPremisesInstrumentedTest` 3 本）
- finish の fix の後（このブランチの HEAD）:
  `cargo test -p ashiato-server st09_` 83 passed・rc=0 / `cargo clippy -p ashiato-server --all-targets -- -D warnings` rc=0 /
  `./collector-android/gradlew -p collector-android detekt :app:assembleDebug :app:testDebugUnitTest` rc=0（615 本・失敗 0）/
  `tools/smoke.sh` rc=0 / `tools/check-log-private.sh` rc=0（`--self-test` OK）/ `tools/check-openapi.sh` OK /
  `check_scenarios.py . st09-media-capture` OK / `tools/check-private.sh` rc=0 / `tools/check-shared-appends.sh` rc=0 / `openspec validate st09-media-capture --strict` OK
- finish の fix の後に `cargo test -p ashiato-server` の全体は走らせていない（変えたのは `media.rs` / `media_tests.rs` だけで、`st09_` の 83 本と smoke で見た）

## 未処置の申し送り

- ST10: 原本の欠け・Wi-Fi 待ちの過去ぶん・動画を不一致に数えない（`docs/handoff/ST10.md`）。D25 の大きさだけの確かめで見落とす置き替えは ST10 の突き合わせで見つかる
- ST23: 本文の消去を D-02 の原本へどう及ぼすか（`docs/handoff/ST23.md`）
- ST30 / ST31: 初回に過去ぶん（約 29 GB 以下）が一度に載る（`docs/handoff/ST31.md`）
- ST38 / ST45: 種類の割当て・渡すときに撮影位置を抜く場所（`docs/handoff/ST38.md` / `ST45.md`）
- 写真を見て 1 件消す口と画面（Q17）: 新しい Story として /requirements から足す
- security review の範囲外の指摘（確かさ 2）: `AndroidManifest.xml` に `android:allowBackup` / `dataExtractionRules` が無く、既定の自動バックアップが有効のまま（差分の前からの仕組み。バックアップに実際に入るかは確かめていない）

🤖 Generated with [Claude Code](https://claude.com/claude-code)
