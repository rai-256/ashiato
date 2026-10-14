# ST14 収集が途切れたら気づける（下流: 実装）

change: `openspec/changes/st14-outage-alert/`。capability は `collection-coverage` だけ。

## `deep.md` に積んだ未決（A）—— **1 件。答えるまで archive しない**

| # | 何を | loss | 問いの HTML |
|---|---|---|---|
| **Q-R16** | 途切れの台帳の `delivered` は「サーバが端末に返した」で、端末が通知を出せなかったとき（携帯で通知の権限が無い・10 秒で打ち切った）も残る。C6 の理由「後から『いつ気づけたか』を数えられる」はこのままでは成り立たない。端末が「出せた」も残すか、C6 の理由を言い直すか | `uncaptured`（台帳は追記のみなので、出せたかどうかは後から作れない） | `docs/briefs/ST14-deep-r3.html` |

推奨は「C6 の理由を言い直す（コードは変えない）」。「出せた」を残すなら API と 2 つの収集アプリに手が入るので、別の Story か `fix/` で持つ。

## 仮決め（B。`design.md` の D（仮）と反転条件）

上流で決めた D5 / D8 / D9 / D10 / D14 / D22〜D25（仮）はそのまま。**下流で足した仮決め**:

| D | 何を仮で決めたか | 反転条件 |
|---|---|---|
| **D26（仮）** | 収集開始日の Asia/Tokyo の 0 時を「最後に生きていた時刻」の下限にする（材料が 1 つも無いソースが永久に判定から外れないように。review/code.md R1） | 最初から権限の無い収集は途切れとして鳴らさなくてよいと言われたとき（開始日を「取得できる状態の材料が初めて届いた日」に絞る） |
| **D27（仮）** | `has_non_collected` を部分索引 `event_non_collected_by_source` で支える（R3） | `has_non_collected` を判定の材料から外すとき、または本人が書いた・導出した記録が増えて索引の書き込みが重くなったとき |
| **D28（仮）** | 達成日（NFR-13）は、取りこぼしの日でも取得できる状態の信号があれば数える。格子では同じ日が③になる（R8） | 格子で③に見える日が達成に数えられるのはおかしいと言われたとき（達成も格子の状態から数える） |
| **D29（仮）** | 携帯も PC も、自分の利用者 ID を `user_id=` に付けて `GET /outages` を問い合わせる。サーバは省略時に既定の利用者のまま（R13） | 1 つのサーバに複数の利用者を置くとき（省略を 400 で断る） |

## Ruling（全 Task の ledger と report から）

- Task 1: 台帳の追記のみトリガは `core.reject_outage_ledger_change()`（この台帳専用）を使う — 書庫の移行が `reject_archive_ledger_change` と `archive_append_only` を落とすので、共有すると `check-immutable.sh` の書庫の戻しが落ちる（D7 は「同じ関数」としていたが、同じ形の別の関数にした。final review の R4 で D7 をこの形に書き直した）— 間違っていたときの費用は小さい（関数名だけ）

ほかの Task の ledger に `Ruling:` と `parked` はない。

## tasks

**11 Task すべて埋めた**（`- [x]` 31 / `- [ ]` 0）。残した Task はない。人間の確認待ち 2 件（物理: 通知の音・実機の未送信の量。st06 R31）は確認バッチの手順に置く。

## 検証（`evidence.jsonl`。HEAD 0c609019 で走らせ直した項目）

| 項目 | コマンド | 終了コード |
|---|---|---|
| 1.2 | `CT st14_reg_`・`tools/check-migrations.sh`・`tools/check-immutable.sh` | 0 / 0 / 0 |
| 7.1 | `CT st14_api_outages_`・`tools/check-openapi.sh` | 0 / 0 |
| 8.1〜8.3 | gradle `OutageNoticeTest` / `OutageNotifierTest` / `OutagePollerTest`・`AlarmManager|WorkManager` の grep・`detekt :app:assembleDebug` | すべて 0 |
| 9.1 / 9.2 | `cargo test -p ashiato-collector-windows outage_notice_*`・windows 向け clippy `-D warnings` | 0 / 0 |
| 9.3 | rsync → `cmd.exe` で Windows 側の `cargo test --test runtime_windows outage`（本物のトースト） | 0 |
| 11.1 | `scripts/quiet-run smoke -- tools/smoke.sh` | 0 |
| 11.2 | `python3 scripts/check_scenarios.py . st14-outage-alert` | 0 |
| 11.3 | fmt・clippy `-D warnings`・`cargo test --workspace`・check-openapi / migrations / immutable / log-private | すべて 0 |

ほかの項目（2.x〜6.x・7.2・10.x）は bc4e76e0 以降の証跡と、code-verify が HEAD 5589c6d4 で全項目を走らせ直した結果（review/code.md の表。31/31 一致）で見ている。

## レビュー

- **whole-branch review**（`code-reviewer.md`）: With fixes（Critical 0 / Important 3 / Minor 6）→ R1〜R10。fixed 9 件（仮決め D26 / D27 / D28）・rejected 1 件（R9: D12 の設計どおり、問い合わせは位置の契機に乗る）。scoped re-review では全部 ADDRESSED
- **code-verify**: R11〜R16。申告 31/31 は HEAD で一致。壊しても緑のまま通っていた値と経路を試験で固定した（R11 `OUTAGE_WARMUP` の 30 分 / R12 携帯の「出せなかったら覚えを進めない」/ R14 本体全体で AlarmManager を使わない / R15 台帳の TRUNCATE の拒否）。
  **R13 は欠陥だった**: 収集アプリが `user_id` を付けていなかったので、利用者 ID が nil でない構成（録画の環境）では、記録が届いているソースまで「途切れ」で返っていた → D29（仮）で直した。R16 は escalated（上の Q-R16）。scoped re-review では R11〜R15 が全部 ADDRESSED
- `review_triage.py . st14-outage-alert`: OK（指摘 41 件・仮決め 10 件・要件へ戻すもの 4 件）

## 申し送り

- **出した**: `docs/handoff/ST47.md` —— D20（正典 `external-ingestion` の 60 日の文）と spec-review R17（書庫の格子の並びの条項に、集計が Must と書庫の間に入ることを書き足す）。ST47 は凍結されているので `followup ST47`
- C1（`export::CATALOG`）は base にも main にも無いので、ST33 の下流が受ける
- 未処置の申し送りはない（`docs/handoff/ST14.md` は開始時から増えていない）

🤖 Generated with [Claude Code](https://claude.com/claude-code)
