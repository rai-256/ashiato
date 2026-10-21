# 仮で進めた決定の確かめ（ST09 / st09-media-capture）

## D25
- 判定: 確か
- 根拠: (1) 失わない: `crates/server/src/media.rs:597-601` で受け取りの行があっても `on_disk` が偽なら `place` に進み 201 を返す（行は `ON CONFLICT DO NOTHING` で 1 つのまま、`:569-580`）。`head_original`（`:784-805`）→ `received_and_on_disk`（`:678-694`）は、ファイルが無ければ 404。端末は 404 で `null`（続ける）→ `put` へ進む（`OriginalUploader.kt:227-233`）ので、置き直しの流れは途切れない。置き場が設定されていなければ HEAD は 404、PUT は 503 で、端末は行を残して試し直す（`:489` の 503 `dir_missing`・`OriginalUploader.kt:233` の `Step.Retry`）—— discarded は起きない。生きている記録が無いときは行だけで答える（`:689-692`）のは記録の削除（D23）と同じ向きで、新たに捨てるものは無い。行を書き換えないので rewrite-all も無く、外へ出すものも無い（exported なし）。CT `media_tests.rs:705 st09_up_put_lost_original_is_asked_again_and_placed_again` がある。
  残る穴（名付けておく。この決定を否にするほどではない）: `on_disk` は中身のハッシュでなく大きさだけを見る（`:665-669`）ので、(a) 別の中身の原本 B が同じ秒・同じ整えた名前で `_<頭8>` の名前に置かれ、B を本人が消し、平の名前に残る A の大きさが B と同じバイト数のとき、B の HEAD は 200 になり端末は行を消す。(b) 本人が同じ大きさの別の中身で上書きしたときも 200。どちらも本人の手作業が前提で、行だけで答える側（もう一方の選択肢）はこれを含むすべての場合に 200 を返すので、選んだ側で失うものは増えない。
  (2) cost: 判定ごとに `core.event` と `core.event_version` を 1 問い合わせ、版の数 × 3 回の `metadata`。経路は ST09 が作る `media.rs` の中だけで、要件の本文・ST10 の口は変えない（design D25 の反転条件で ST10 へ移す形は書いてある）。高くつかない。
  `core.event_version` を `_live` 越しでなく直に読む（`migrations/202609120943_version_and_ledger.sql:13` は「`_live` 越しにだけ」）が、使うのは名前の候補を組むことだけで値を外に返さない（応答は 200/404 のみ）。漏れ・失いの 4 つのどれにも当たらない。
  (3) follows Q12: Q12 の答え（年/月のフォルダに撮影日時と元の名前。`deep-answers-1.txt:14`）は本人がエクスプローラで D-02 を開く前提で、D25 はその「開いて動かす・消す」に備える。`on_disk` は `layout` の同じ 3 候補（`:276-293`）を見るので、並びの規則と同じ向き。Q12 の「並びは作り直せる（ファイルを動かすだけ）」も、動かした先が新しい規則の名前なら見つかり、見つからなくても置き直すだけで記録は書き換えない。
  (4) rests_on: D10（並び・D10a の整え方・受け取りの行）で足りる。ほかに効いている土台は既存の版の表（`core.event_version`。ST の土台で決定済み）と、端末の 200 で行を消す振る舞い（R9。`OriginalUploader.kt:231`）で、どちらも仮の決定ではない。

## D26
- 判定: 確か
- 根拠: (1) 失わない: `media.rs:42,44` の `PRIVATE_DIR_MODE = 0o700` / `PRIVATE_FILE_MODE = 0o600` を、既定の置き場（`prepare` `:163-170`）、`.incoming/`（`:500` `private_dir_builder(false)`）、年/月（`place` `:724` `private_dir_builder(true)`）、`.part`（`:506-509`。`rename` で原本がそのまま引き継ぐ）に当てる。権限を絞るだけでバイトは捨てず（discarded なし）、取るものも減らさず（uncaptured なし）、既にあるものの権限も書き換えない（rewrite-all なし）。選んだ側は exported を止める向き。S-01 は自宅 PC で本人のアカウントで動く（`docs/requirements.md:49`。専用の利用者で動かす systemd の unit 等は repo に無い —— `grep -rln 'User=\|systemd' tools docs` は 0 件）ので、本人が Q12 のとおりフォルダを開いて読むことは変わらない。CT `media_tests.rs:764 st09_up_put_original_is_readable_only_by_its_owner` がある。
  Windows では何もしない（`#[cfg(unix)]`）。既定の `%LOCALAPPDATA%` は持ち主だけの ACL だが、別のドライブを設定したときは親の ACL を継ぐ —— これは選んだ側が新たに漏らすものではなく（前と同じ）、`chosen` が「Unix では」と範囲を書いている。
  (2) cost: 書き出し（`export.rs:1008-1030` の `ensure_dir` / `create_private_file`）と同じ形で、Story の範囲を越えず、要件の本文も変えない。共有したいときに設定が要るのは `cost` に書いてあるとおり。
  (3) follows Q12: Q12 は並びの問いで、本人が開くことを前提にする。持ち主は本人なので向きは同じ。
  (4) rests_on: D10（置き場・`.incoming/`・`.part` の作り方）で足りる。既定の置き場だけ作り設定された置き場は作らない（D10。spec の Scenario で固定）も同じ D10 の中。
