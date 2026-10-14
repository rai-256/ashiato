# ST33 コードレビュー — st33-data-export

## final review（6d7d65f..b74736b）

席: final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`。review package `.superpowers/sdd/tasks/review-6d7d65f7..b74736b0.diff`）。
判定: **With fixes**（Critical 0 / Important 4 / Minor 12）。本人の決定（Q1〜Q9）と R2（削除済みを出さない）・C3（0700 / 0600）は構造とテストで担保されている、という評価。
Task の ⚠️ のうち T2（`App::with_export`）・T4（LateMark の結線と失敗の差し込み）・T5 F7（`Progress` を毎回作る）・T6（`docs/openapi.json` の `/export`）・T8（24px は e2e 9.5 で実測）・T9（DB を作り直せば e2e 全体 1.7 分）は解消を確かめた。

## R1. `GET /export` が読むたびに除く範囲の CTE を全件で評価し直し、画面は 1 秒ごとに前の応答を待たずに重ねて読む
- 分類: Important / production readiness（性能・プールの枯渇）
- 成果物: crates/server/src/export/ledger.rs:373（`erased_after`）、crates/server/src/export/job.rs（`read_status`）、web/src/SettingsView.tsx:84-87
- 根拠: `overlap_event` は `event_live` の全行に座標の判定と (c) の相関副問い合わせを当てる。Task 3 の報告で共有 DB 1 回約 10 秒、Task 5 の報告で座標つき 1 行約 8ms。`setInterval` は前の応答を待たない。プールは `max_connections(5)`（lib.rs:2840）で 1 本は写しが握るので、残り 4 本を状態の読み出しが奪い、取り込み（`POST /events`）を詰まらせうる。D17（仮）の前提「1 秒ごとでも軽い」が成り立たず、D17 自身の反転条件「重くなれば（`last` の `export_id`・削除の台帳の最後の行）で結果を覚える」に当たる
- kind: technical
- 提案: `erased_after` を（`last` の `export_id`, 削除の台帳の最後の行, 作り直しの印の最新）の鍵で覚える。画面は応答を待ってから次を読む `setTimeout` の連鎖にする。design D17 の（仮）を書き換える

## R2. 書いた zip の大きさかハッシュを取れなかったとき、名前の付いた zip が誰にも見えないまま残る（T6 F3 の未修正）
- 分類: Important / correctness（本人の決定 Q4「1 つだけ置く」が黙って崩れる）
- 成果物: crates/server/src/export/job.rs（`finish_done` の `_ =>` の枝）
- 根拠: `rename` の後に `insert_failed` を書く。そのファイルは `done` でないので `removable_previous` にも `leftover` にも出ず、`finish` を持つので `interrupted` にも出ない
- kind: technical
- 提案: その枝で `remove_file` してから `insert_failed` を書く（「失敗したら残さない」と揃える）。失敗を差し込むテストを 1 本

## R3. 失敗・止まった・消せなかったファイルの 1 行が「前回の書き出し」の節にあり、spec と D13 の「書き出しの節」と違う（T8 ⚠️）
- 分類: Important / plan alignment
- 成果物: web/src/SettingsView.tsx（`lines` が `export-last` の section の中）、web/e2e/settings-export.spec.ts（`getByTestId("export-line")` が節を絞らない）
- 根拠: spec の 543〜547 行と design D13 は「書き出しの節に」と書き、D13 は「全記録の書き出し」と「前回の書き出し」を別の節に定める
- kind: technical
- 提案: `lines` を `export-section` へ移し、e2e の locator をその節に絞る

## R4. 1 年分（35 万行）の量を測っておらず、tasks 5.6 の「5 分を超えたら D5 を考え直す」が未判定
- 分類: Important / plan alignment（5.6 の判定が残っている）
- 成果物: crates/server/src/export_file_tests.rs（`export_volume`）、tasks.md 5.6
- 根拠: `export_volume` は既定の 2 万行（`c02-window`、座標つき 1%）しか走っていない。Task 5 の報告は座標つき 1 行約 8ms で、35 万行で 5 分を超える見込みと書く（`.superpowers/sdd/st33-task-5/task-5-report.md:48`）
- kind: technical
- 提案: 使い捨ての DB で `ASHIATO_EXPORT_VOLUME_ROWS=350000` を 1 回走らせ、時間と最大メモリを記録する。5 分を超えたら PR 本文に既知の課題として残し、R1 と合わせて判定する

## R5. `excluded_sql` の `erased_as_of: Some(T)` は使われない分岐で、doc が事実と違う（T3 ⚠️ の再分類）
- 分類: Minor / code quality
- 成果物: crates/server/src/export/excluded.rs:103-110
- 根拠: 本番の 3 か所とテストはすべて `None`。D17 は spec-review R25 に従いブルームフィルタで実装されている。doc の「D17 が写しの時点 `T` で使う」は事実と違う
- kind: technical
- 提案: 引数ごと消す

## R6. 書く途中で失敗したとき `.partial` を消す経路にテストが無い（T5 ⚠️）
- 分類: Minor / testing
- 成果物: crates/server/src/export/（`write_zip` の `if result.is_err() { remove_file }`）、crates/server/src/export_tests.rs:290
- 根拠: 既存のテストは `Stage::Write` の前で落とすので `.partial` がまだ無い
- kind: technical
- 提案: D2 の差し込み口（`before_file`）で stream の途中から失敗させ、`.partial` が残らないことを確かめる

## R7. handoff C1 の安全網「`core.source` の全ソースが緯度経度の有無と幅の読み方に明示で置かれている」試験が無い
- 分類: Minor / testing（申し送り）
- 成果物: crates/server/src/export/excluded.rs（`NO_COORD_SOURCES` / `SPAN_RULES`）
- 根拠: docs/handoff/ST33.md の C1。「緯度経度を持つ」側が暗黙の既定なので、ST17 の `s01-feeling` のように一覧に無いソースが足されても名前つきで落ちない。ST17 は未 archive（片割れは docs/handoff/ST17.md:29）
- kind: technical
- 提案: 緯度経度を持つ側も明示の一覧にし、`core.source` の全行がどちらか一方にだけ置かれていることを DB と突き合わせる試験を置く（ST17 がソースを足した時点で名前つきで落ちる）

## R8. design.md が実装からの逸脱を記録していない
- 分類: Minor / documentation
- 成果物: openspec/changes/st33-data-export/design.md（D5 の (c)、D3 / D7 の `.partial` の名前）
- 根拠: D5 の (c) は `lead()` の窓関数のはずが行ごとの相関副問い合わせ（T3 F3）。`.partial-…-<8 桁の 16 進>.zip` の接尾辞が design に無い
- kind: technical
- 提案: design に実装の形と理由を書く

## R9. 古い doc が残っている
- 分類: Minor / documentation
- 成果物: crates/server/src/export.rs:4-5（「この段階の本体は…空の zip」。T5 F6）、crates/server/src/lib.rs:61-62（「書き出しの台帳の移行と錠」が `export_file_tests` に付いている。T5 F5）
- kind: technical
- 提案: いまの形に合わせて書き直す

## R10. テストがプロセス全体の `TMPDIR` を `set_var` で書き換える
- 分類: Minor / testing（並行のテストとの競合）
- 成果物: crates/server/src/export_file_tests.rs:596-612（`TmpdirGuard`）
- 根拠: `EXPORT_SERIAL` が直列にするのは書き出しのテストだけ。並んで走る他のテストは差し替えた `temp_dir()` を見る。glibc の getenv / setenv は並行に呼ぶと競合する
- kind: technical
- 提案: `TMPDIR` を差し替えず、プロセスの一時フォルダに新しくできたファイルだけを見る形にする（同じテストはもう `process_tmp` を見ている）

## R11. `cargo test --workspace` で `drops_tests::drops_api_idempotent` が 2 回 `40P01 deadlock` で落ちている（T10 F3）
- 分類: Minor / testing
- 成果物: crates/server/src/drops_tests.rs
- 根拠: 書き出しのテストの失敗は無い。共有 DB で移行を当て直す既存のテストの形が原因と見られるが、この branch が頻度を上げたかは特定できていない
- kind: technical
- 提案: 原因を特定できなければ PR 本文に既知の揺れとして残す

## R12. 失敗を黙って捨てている（T6 F2）
- 分類: Minor / error handling
- 成果物: crates/server/src/export/job.rs（`delete_previous` と `finish_done` の `let _ =`。`spawn_blocking` の JoinError、`insert_failed` の結果）
- kind: technical
- 提案: `tracing::warn!(kind = …)` を 1 行ずつ足す（値は載せない）

## R13. `GET /export` が読むたびに `leftover` の候補の SHA-256 を全体で計算する（T6 F4）
- 分類: Minor / performance
- 成果物: crates/server/src/export/job.rs
- kind: technical
- 提案: R1 と同じく、（パス・大きさ・更新時刻）が同じなら前の結果を使う

## R14. `last_failure.kind` を `reason` の文字列を `:` で割って取っている（T6 F6）
- 分類: Minor / type safety
- 成果物: crates/server/src/export/job.rs
- kind: technical
- 提案: 台帳の行から種別を構造で取る（列か、`reason` を作る側と読む側で同じ関数を使う）

## R15. `messages.ts` の `sourcesTitle` が使われていない（T8 F2）
- 分類: Minor / code quality
- 成果物: web/src/messages.ts
- kind: technical
- 提案: 消す

## R16. `export_post` の `password` が `serde_json::Value` と `Bytes` の写しに残り、zeroize されない
- 分類: Minor / security（できる範囲での消去）
- 成果物: crates/server/src/lib.rs（`export_post`）
- 根拠: D の決定には反しない（できる範囲）
- kind: technical
- 提案: `serde_json::Value` を経ず、`Zeroizing<String>` を持つ型へ直接読む
