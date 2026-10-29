# ST51 仮で進めた決定の確かめ（deep-review provisional、2026-10-29）

## D1
- 判定: 確か
- 根拠: 失わない側（すべての生存信号に積んだ時の版）で 4 つの loss は起きない。生存信号は溜めの上限の対象外 —— C-01 `collector-android/app/src/main/kotlin/dev/ashiato/collector/LocationService.kt:314`（「上限の対象にしない（捨てない）」）・`docs/collector-contract.md:525`、C-02 `crates/collector-windows/src/outbox.rs:4`（「上限も破棄も持たない」）。S-01 は生存信号の行を消せない（`migrations/202609111112_immutable_heartbeat.sql:32-35` の BEFORE UPDATE OR DELETE。`grep -rn "DELETE FROM core.heartbeat" migrations crates/server/src` は注釈 1 件だけ）。版を原文に入れても冪等キーは崩れない —— 溜めた行は積んだときの原文のまま送るので、同じ信号を送り直しても鍵は同じ（design D6・D12、specs/desktop-collection/spec.md:40・specs/device-collection/spec.md:45）。外に出る値は組み立ての印だけで私的な中身ではない（design D12）。費用: 積む間隔は C-01 `Heartbeat.kt:22` の `HEARTBEAT_INTERVAL_MS = 6 時間`（1 日 4 件）で、「ソースの数 × 1 日 4 件」と合う。要件の本文は変えない（FR-121 `docs/requirements.md:1017`・`:1027` は版の申告を生存信号に持たせると書いている）。follows・rests_on は無く、土台になる仮の決定も無い（版を欄と原文の両方に入れるのは仮でない D6）

## D2
- 判定: 確か
- 根拠: 書く口を増やす側で、行を捨てる・外に出す・書き直すことは起きない。置き場の順はコードと合う —— migrate は `crates/server/src/lib.rs:295-` で 接続 → 錠 → 読む → 終了コード 3 → 退避 → 当てる、起動は `lib.rs:3862` の `startup_refusal` が通ってから `:3933` 付近の bind・serve。design D2 の「検査を通った後・待ち受けの前」に書けば、拒んだ回（終了コード 3）は書かない。直前の行の版と比べて違うときだけ書く（D7 の取引と錠）ので、行が増えすぎることもない。費用は起動ごとの問い合わせ 1 本と、実行時の役割への INSERT 1 つ。今は `crates/server/src/grants.sql:6` が `core` の全部の表に SELECT・INSERT・UPDATE・DELETE を付けているので、INSERT は新しく渡す権限ではない（D10 で UPDATE・DELETE を外す）。既存の役割の試験が直しになる（agreement.json の IMP-05。本人が合意済み）が、Story の範囲を越えない。FR-117（`docs/requirements.md:1009`）・FR-121（`:1017`）の本文は変えない。土台になる仮の決定は無い（前の版の求め方は仮でない D7）

## D3
- 判定: 確か
- 根拠: 失わない側（`core` の追記のみの表 + 書き出しに分類）で、`exported` に当たるのは版の文字列・結果・当てた版の名前・SQLSTATE だけ。誤りの本文は Q1 の答え（deep-answers-1.txt）で残さないので、私的な値は外に出ない（design D3 の反転条件は起きない）。`core` に置けば書き出しの分類の検査に必ず載る（`crates/server/src/export.rs:629-631` は `core` の BASE TABLE を全部走査する）。書き換えの門・権限は D10 が `core.reject_heartbeat_rewrite`（`migrations/202609111112_immutable_heartbeat.sql:15-35`）と同じ形で持つ。費用は表 1 つと分類 1 行で、Story の範囲内（ST51 の Goals）。補足（判定は変えない）: keeps の「DB を書き出しから作り直しても履歴が残る」は書き過ぎ —— 書き出しから D-01 へ取り込む口は今のコードに無い（`grep -n restore crates/server/src/*.rs` で見つかるのは `deletion.rs:99` の削除の取り消しだけ）。失わない理由は「書き出しのファイルに残る」で足りる

## D4
- 判定: 確か
- 根拠: 形を固定する側で、凍結した行の書き直し（`rewrite-all`）は起きない。形の表は `specs/distribution/spec.md:126`（`update_history` の 10 列の名前・型・NULL の可否）、`:128`（`component`・`via`・`outcome` の値の一覧）、`specs/collection-coverage/spec.md:5`・`:13`（`client_version` → 列 `collector_version` text NULL）にある。後から足すのは列の追加（今の行は NULL）と値の一覧の追加（CHECK を付け直す。今の行は新しい制約も満たす）だけで、今の列・値を変えない（design D4）。費用は Requirement の表と information_schema を突き合わせる試験 1 本で、範囲内。要件の本文は変えない。rests_on は空だが、表の置き場（D3）と `via` の値（D2 の 2 つの口）を固定する決定なので、実際は D3・D2 に乗っている。D2・D3 も確かと判定したので判定は変えない。ただ確認バッチで D2 か D3 が「変える」になったときは D4 の表も見直す（台帳の rests_on に D2,D3 を足すのが正しい）

## D5
- 判定: 確か
- 根拠: 導く側（ビュー）で、変化の行は凍結されない。入力の生存信号の行は消せず書き換えられない（`migrations/202609111112_immutable_heartbeat.sql:32-35`、`crates/server/src/deletion.rs` に heartbeat の語は無い）。なので並べ方（Q4）を後で直しても、過去の分まで出し直せる。D-01 の中で `discarded` も `rewrite-all` も起きない。`exported` は書き出しの `collector_version_changes.jsonl`（`specs/data-durability/spec.md:47`）だけで、版の文字列と端末・時刻の行。同じ値は `heartbeats.jsonl` の `collector_version` にも出る（`:48`）ので、新しく外に出るものは無い。費用は読むたびの窓関数 1 本（1 端末 1 日 4 件 × ソースの数）で、範囲内。要件の読み替えは本人が Q3 で選んだ（deep-answers-1.txt の Q3「生存信号の行から導く …D-01 のビューと全記録の書き出しに出す」。review/deep.md R6）。補足（判定は変えない）: (1) D5 は本人の Q3 の答えそのもので、もう仮ではない。台帳では follows が null だが、実際は follows Q3。`provisional.py resolve`（理由: 本人が Q3 で答えた）の対象になる。(2) design D1 の反転条件が「そのときは D5 の前提を見直す」と書いていて、D5 は D1 に乗っている。rests_on に D1 が無い（D1 も確かと判定したので判定は変えない）
