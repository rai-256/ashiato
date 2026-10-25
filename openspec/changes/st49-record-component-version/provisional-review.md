# ST49 仮で進めた決定の確かめ

## D1
- 判定: 確か
- 根拠: 収集した記録の中身を書き換える経路は `apply_external_update` だけ（`grep -rn "UPDATE core.event" crates/server/src` の試験でない行は、削除の印（`deletion.rs:179/282/290`・`stay_store.rs:900/1040`）・水位（`lib.rs:1589`）・滞在（`stay_store.rs:971`）と、`lib.rs:1667` の更新だけ）。`lib.rs:1625-1651` は前の版へ列を名指しで写してから本表を書き換えるので、名指しに 3 列を足せば前の値は `core.event_version` に残る（uncaptured / discarded は起きない）。`advance_watermark`（`lib.rs:1578-1597`）と `SkippedStale` / `SkippedDeleted`（`lib.rs:1612-1621`）は版に触らない。D9 の `Internal`（C-03）による更新で `collector_version` が NULL になっても、前の値は前の版に残る。`Stored` / `load_stored` に 3 列を足すことは tasks 4.2 にあり、前の版の INSERT は tasks 3.2 にある。cost は列の名指しと読み口の作り直しだけで、要件の本文を変えない（record-envelope spec:137 がすでに「今の中身を作った部品の版」と書いている）。`follows` は無い。土台の決定は D9・D10（どちらも仮でない）

## D2
- 判定: 確か
- 根拠: `stay_store.rs:593-601` は `e.raw != raw` のときだけ `push_version` してから `update_stay` を呼ぶ。`push_version`（`stay_store.rs:927-942`）は `core.event` から列を名指しで `INSERT … SELECT` するので、3 列を足せば書き換える前の `server_version` が前の版に写る。印だけの書き換え（`e.raw == raw`）・`absorb`（`stay_store.rs:1040`）・遅れて届いた記録による消去（`stay_store.rs:900`）は削除の印だけを書き換え、滞在の行を消さない（「行は消さない」D3 の注記）。滞在は `origin = 'derived'` なので門（`202609120944_gates.sql` の `IF OLD.origin <> 'collected' THEN RETURN NULL`）にかからず、D21 の影響も受けない。cost は `insert_stay`・`update_stay`・`push_version` の列を足すことだけ。`follows` は無い。`rests_on` は空でよい（D1 とは別の経路で、D1 の答えに依らない）

## D3
- 判定: 確か
- 根拠: 失わない側（知らない欄を捨てずに送る）を選んでも、新しく生まれる uncaptured / discarded / exported / rewrite-all は無い。送り返した欄で S-01 が断る型は無い（`grep -rn deny_unknown_fields crates/server/src` は `RebuildRequest`・`EraseRequest`・`RestoreRequest`（`lib.rs:1833/1861/1882`）だけで、取り込み・破棄の報告・原本の欠け・生存信号の型には無い）。ST49 より前の版へ戻した間の脱落と、入れ替えの順を守らなかった間の脱落は、どちらを選んでも起きる（chosen が代償として書いている）。cost は「送り出しの 1 か所ずつ」より広い: C-01 は `SegmentStore` の `decode` / `lineOf`（`SegmentStore.kt:409-427`）、`Outbox` の `head` / `snapshot`、`Sender.flush` の `encodeToString(ListSerializer(serializer), batch)`（`Sender.kt:176`）が同じ総称の部品で、記録・生存信号・破棄の報告（`DropReport.kt:136`）・原本の欠け（`OriginalLedger.kt:364`）の 4 つの溜めに効く。それでも C-01 の溜めと送り出しの中で閉じていて、Story の範囲を越えず、要件の本文を変えない。tasks の 9.x（`tasks.md:182`）と 8.x（`tasks.md:162`）に作業と試験（`unknownFieldSurvivesResend`・`st49_c02_unknown_field_survives_resend`）がある。`follows` は無い。土台の D13 は仮でない

## D4
- 判定: 否
- 根拠: 条件 1（exported）と条件 4（`rests_on` から D1 が抜けている）。D4 が決める書き出しの説明（design.md の D4: `collector_version` は「**記録を作った**収集アプリの組み立ての印」、`web_version` は「**記録を組んだ**画面の組み立ての印」）は、D1（仮）の「版は今の中身を作った部品の版」（record-envelope spec:137、design D1）と合わない。外部サービス由来の更新で版が動いた行では、書き出しの値は更新した部品の版なのに、説明は最初に作った部品と読める。書き出しは外に出るので、出た説明は後から直せない（D4 自身の keeps の理由と同じ）。D4 の説明の文言は D1 の答えに依る（D1 を反転条件どおり「最初に作った版」に戻すと、いまの文言のほうが合う）のに、`rests_on` が `[]` になっている。「申告（S-01 は確かめていない）」と「S-01 が自分で入れた」の書き分けそのものは、弱い側から強める向きで失うものが無い。否になるのは、説明の「記録を作った」の部分

## D21
- 判定: 確か
- 根拠: 門の対象にする側を選んでも失うものは無い。門は書き換えを断るだけで、行と値を消さない（`202609120944_gates.sql` の `core.require_version_or_ledger` は `RAISE EXCEPTION` でまとまりを落とす）。版を書き換える経路は `apply_external_update` だけで、同じまとまりで前の版を積む（`lib.rs:1625-1651`）。前の版の INSERT が門と同じ移行で 3 列を写すことは tasks 3.2 にある。照合を `IS NOT DISTINCT FROM` にすることは design D21 にあり、今の照合（`v.raw = OLD.raw` など）の `=` を写すと版の無い記録の更新がすべて落ちる、という落とし穴を避けている。本文の消去（`is_erasure`）は版に触らない（D16）ので、門に版の列を足しても消去は今の分岐で通る。cost は門の関数の本体を写して 3 列を足すこと（移行は `CREATE OR REPLACE`。D19 で ST41 と写し直しを取り合う）で、要件を変えない。`rests_on: ["D1"]` は合っている。D1 が「今の中身を作った版」でなくなっても、門は前の版を求めるだけなので向きは変わらない
