# ST14 収集が途切れたら気づける（下流: 実装）

change: `openspec/changes/st14-outage-alert/`。capability は `collection-coverage` だけ。

## `deep.md` に積んだ未決（A）—— **なし**

下流で積んだ Q-R16（途切れの台帳の `delivered` は「サーバが端末に返した」で、端末で出せたかは残らない。loss `uncaptured`）は
**2026-10-15 に本人が答えた**（`deep-answers-6.txt`）: C6 の理由を「いつ端末に返したか」へ言い直す。端末で出せたかは残さない（`uncaptured` を受け入れた）。
deep.md の C6・`specs/collection-coverage/spec.md` の本文・`go.json` を言い直した。Requirement の SHALL・Scenario・コードは変えていない。

## 仮決め（B。`design.md` の D（仮）と反転条件）

上流で決めた D5 / D8 / D9 / D10 / D14 / D22〜D25（仮）はそのまま。**下流で足した仮決め**:

| D | 何を仮で決めたか | 反転条件 |
|---|---|---|
| **D26（仮）** | 収集開始日の Asia/Tokyo の 0 時を「最後に生きていた時刻」の下限にする（材料が 1 つも無いソースが永久に判定から外れないように。review/code.md R1） | 最初から権限の無い収集は途切れとして鳴らさなくてよいと言われたとき（開始日を「取得できる状態の材料が初めて届いた日」に絞る） |
| **D27（仮）** | `has_non_collected` を部分索引 `event_non_collected_by_source` で支える（R3） | `has_non_collected` を判定の材料から外すとき、または本人が書いた・導出した記録が増えて索引の書き込みが重くなったとき |
| **D28（仮）** | 達成日（NFR-13）は、取りこぼしの日でも取得できる状態の信号があれば数える。格子では同じ日が③になる（R8） | 格子で③に見える日が達成に数えられるのはおかしいと言われたとき（達成も格子の状態から数える） |
| **D29（仮）** | 携帯も PC も、自分の利用者 ID を `user_id=` に付けて `GET /outages` を問い合わせる。サーバは省略時に既定の利用者のまま（R13。携帯の組み立ては R17 で試験に通した） | 1 つのサーバに複数の利用者を置くとき（省略を 400 で断る） |

## Ruling（全 Task の ledger と report から）

- Task 1: 台帳の追記のみトリガは `core.reject_outage_ledger_change()`（この台帳専用）を使う — 書庫の移行が `reject_archive_ledger_change` と `archive_append_only` を落とすので、共有すると `check-immutable.sh` の書庫の戻しが落ちる（D7 は「同じ関数」としていたが、同じ形の別の関数にした。final review の R4 で D7 をこの形に書き直した）— 間違っていたときの費用は小さい（関数名だけ）

- Task 9（**parked**。review/code.md R18 の PC 側）: PC の `toast.rs` には `ToastNotifier.Setting()` の確かめを足さない —— 本人が設定で差出人の通知を切っていても `Show` は `Ok` を返し、覚えが進む疑いがある。
  ただし試験で「切られた」状態を作れない（登録簿の `Enabled = 0` は `Setting()` に反映されず、一度も出していない差出人では `Setting()` が `Err`。Windows 側の cargo で実測 2026-10-15）。
  足した形は初めての通知を止める危険があり、試験で固定できない。反転条件: `Setting()` が `Disabled*` を返す状態を試験（または確認バッチの実機）で作れる手段が見つかったとき。携帯側（チャネル）は 8.2 で直した

ほかの Task の ledger に `Ruling:` と `parked` はない。

## tasks

**11 Task すべて埋めた**（`- [x]` 31 / `- [ ]` 0）。残した Task はない。人間の確認待ち 2 件（物理: 通知の音・実機の未送信の量。st06 R31）は確認バッチの手順に置く。

## 検証（`evidence.jsonl` と code-verify の走らせ直し）

最後の fix（0dcbe8f6）が触ったのは携帯（8.x）だけなので、8.1〜8.3 を `scripts/verify-run` で HEAD 0dcbe8f6 で走らせ直した。
ほかの項目は code-verify が HEAD 122502c7（main #91 の上へ rebase した後）で全項目を走らせ直した結果（review/code.md の表。31/31 一致・すべて rc=0）。

| 項目 | コマンド | 終了コード |
|---|---|---|
| 1.x〜7.x | `CT st14_*`・`coverage::tests::`・`api_tests`・check-openapi / migrations / immutable・vitest | すべて 0（122502c7） |
| 8.1〜8.3 | gradle `OutageNoticeTest` / `OutageNotifierTest` / `OutagePollerTest`・`AlarmManager|WorkManager` の grep・`detekt :app:assembleDebug` | すべて 0（**0dcbe8f6**。単体 448 本・failures 0） |
| 9.1 / 9.2 | `cargo test -p ashiato-collector-windows outage_notice_*`・windows 向け clippy `-D warnings` | 0 / 0（122502c7） |
| 9.3 | rsync → `cmd.exe` で Windows 側の `cargo test --test runtime_windows outage`（本物のトースト） | 0（122502c7。PC のコードは 0dcbe8f6 でも同じで、2026-10-15 に再度 1 passed） |
| 10.1〜10.5 | stack → `/coverage` の `jq -e`・vitest・e2e（本物の Chromium）・lint / tsc / build | すべて 0（122502c7） |
| 11.1 | `scripts/quiet-run smoke -- tools/smoke.sh` | 0（122502c7） |
| 11.2 | `python3 scripts/check_scenarios.py . st14-outage-alert` | 0（122502c7） |
| 11.3 | fmt・clippy `-D warnings`・`cargo test --workspace`・check-openapi / migrations / immutable / log-private | すべて 0（122502c7） |

## レビュー

- **whole-branch review**（`code-reviewer.md`）: With fixes（Critical 0 / Important 3 / Minor 6）→ R1〜R10。fixed 9 件（仮決め D26 / D27 / D28）・rejected 1 件（R9: D12 の設計どおり、問い合わせは位置の契機に乗る）。scoped re-review では全部 ADDRESSED
- **code-verify**: R11〜R16。申告 31/31 は HEAD で一致。壊しても緑のまま通っていた値と経路を試験で固定した（R11 `OUTAGE_WARMUP` の 30 分 / R12 携帯の「出せなかったら覚えを進めない」/ R14 本体全体で AlarmManager を使わない / R15 台帳の TRUNCATE の拒否）。
  **R13 は欠陥だった**: 収集アプリが `user_id` を付けていなかったので、利用者 ID が nil でない構成（録画の環境）では、記録が届いているソースまで「途切れ」で返っていた → D29（仮）で直した。R16 は escalated → 答え済み（上の Q-R16）。scoped re-review では R11〜R15 が全部 ADDRESSED
- **code-verify（rebase 後・HEAD 122502c7）**: R17・R18。申告 31/31 は一致。
  **R17**: 携帯が問い合わせに利用者を付ける組み立てを、どの試験も通っていなかった → 差し替える口を打つ関数だけに狭め、実際に打たれた文字列を見る試験で固定（D29（仮））。
  **R18 は欠陥だった**: 本人が「収集の途切れ」のチャネルだけを切ると、携帯は何も出さずに「出せた」と返し、Q4 の 1 回の音を使い切っていた → チャネルが切られていたら出せなかった扱い（8.2）。PC 側は上の Task 9 の Ruling で park。
  scoped re-review では R17・R18（携帯）が ADDRESSED、PC の park は根拠つきで妥当
- `review_triage.py . st14-outage-alert`: OK（指摘 43 件・仮決め 11 件・要件へ戻すもの 4 件）

## 申し送り

- **出した**: `docs/handoff/ST47.md` —— D20（正典 `external-ingestion` の 60 日の文）と spec-review R17（書庫の格子の並びの条項に、集計が Must と書庫の間に入ることを書き足す）。ST47 は凍結されているので `followup ST47`
- C1（`export::CATALOG`）は base にも main にも無いので、ST33 の下流が受ける
- 未処置の申し送りはない（`docs/handoff/ST14.md` は開始時から増えていない）

🤖 Generated with [Claude Code](https://claude.com/claude-code)
