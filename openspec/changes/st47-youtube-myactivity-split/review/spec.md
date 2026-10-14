# ST47 spec-review（上流の成果物の独立レビュー）

対象: `openspec/changes/st47-youtube-myactivity-split/`（proposal.md / specs/external-ingestion/spec.md / design.md / tasks.md）。
突き合わせたもの: 同じ change の `deep.md`、`docs/stories/ST47.md`、`docs/stories/INDEX.md`、`openspec/specs/external-ingestion/spec.md`（正典）、
`crates/server/src/archive/`（classify.rs / worker.rs / youtube.rs / mod.rs）、`archive_tests.rs`、`tools/archive-shape.sh`、`tools/smoke.sh`、`migrations/`。

## 機械の検査（最初に写す）

| コマンド | 結果 |
|---|---|
| `openspec validate st47-youtube-myactivity-split --strict` | `Change 'st47-youtube-myactivity-split' is valid`（rc=0） |
| `python3 scripts/check_chain.py .` | `chain: OK (0 件 / 未回収 0 件 / warn 0 件)`（rc=0。観点 8 の再生成との一致も通る） |
| `python3 scripts/check_scenarios.py . st47-youtube-myactivity-split` | `scenarios: FAIL (担保なし 22 件)`（rc=1）。22 件はすべてこの change が足した Scenario で、上流の段では試験が無いので想定どおり（Task 7.2 が閉じる）。warn 8 件（`archive_flow_tests.rs` などの spec に無い Scenario を指す印）は ST12 から残っているもので、この change とは関係ない |
| `python3 scripts/review_triage.py . st47-youtube-myactivity-split` | `triage: OK`（指摘 6 件。`review/deep.md` の分） |

MODIFIED の Requirement は、正典にある Scenario 12 本をすべて逐語で写している（`diff` で確かめた。差は足した本文 5 行・導出元・理由 1 段だけ）。

---

## R1. D6（仮）が Q4 の答えを一部覆している —— `languages: []` に印を 1 回置くと、表に無い言語のファイルは以後すべて確認待ちにならない
- 成果物: openspec/changes/st47-youtube-myactivity-split/design.md（D6）/ specs/external-ingestion/spec.md
- 根拠: deep.md:68-70（Q4 の本人の答え「表が 1 件も当たらない言語のファイルは確認待ちに戻す」）。design.md:167（YouTube の同一性は `{"kind","products","languages"}`）と design.md:174（`languages: []` の形にも `--confirm` で印を置ける）を合わせると、韓国語・フランス語など表に無い言語のファイルはどれも同じ `{"kind":"YouTube","products":["YouTube"],"languages":[]}` のハッシュになる。1 回 `[]` に印を置くと、後から来た別の言語のファイルもその印で格納され、その言語の評価は視聴に入って凍結される。spec.md:113 は「表の当たった言語の形に置いた印では格納せず」としか書いていないので、`[]` への印が後のファイルを通すことを spec は止めない。D6 の理由（design.md:175）は合成の材料（`tools/smoke.sh:151-157` の題名の無い `watch-history.json`）を通すためで、本人の暮らしの理由ではない
- kind: conflict
- loss: rewrite-all
- 提案: (仮)にせず本人に A として返すか、「既定は厳しい側」の C にする（`--confirm` は `languages` が空の YouTube の形を断る。smoke と合成の材料には決まり文句を持つ題名を付ける）。どちらにするかを spec の Requirement と Scenario に書く（いまは design にしか無い）
- 処置: escalated — deep.md の第 2 回 Q7（R1）。推奨（`--confirm` が言語の空の YouTube の形を断る）を design D6・spec・tasks 3.2 に書いてある

## R2. 要件本文は「評価の決まり文句の表が当たった言語」、D4 は「視聴・検索の決まり文句でも言語を見分ける」で、評価の無いファイルの扱いが逆になる
- 成果物: openspec/changes/st47-youtube-myactivity-split/specs/external-ingestion/spec.md
- 根拠: spec.md:110 / 112 / 113 / 125 はどれも「**評価の**決まり文句の表が当たった言語」「**評価の**決まり文句の表が 1 件も当たらない」。design.md:153-157 は視聴・検索の決まり文句も言語の見分けに使うとし、その理由を「評価だけで見ると、評価の無い書き出しのたびに確認待ちに戻る」と書いている。Scenario spec.md:192-195 は「表の言語の決まり文句」で、design の側に寄っている。評価を 1 件も持たない日本語のファイルは、spec.md:113 の文言どおりなら確認待ちになり、D4 なら `["ja"]` で格納される。試験は D4 に合わせて書かれるので、要件本文の主張は真にならない
- kind: technical
- 提案: 要件本文の「評価の決まり文句の表」を「決まり文句の表（評価・視聴・検索）」に直す。Scenario `評価を持たない日本語の YouTube のファイルは日本語の形の印で格納される` を 1 本足す
- 処置: fixed 3.1 — spec の本文の「評価の決まり文句の表」を「決まり文句の表（評価・視聴・検索）」に直し、Scenario `評価を持たない日本語の YouTube のファイルは日本語の形の印で格納される` を足して tasks 3.1 に割り当てた

## R3. Scenario「YouTube 以外の形に置いた印は効いたままになる」は、ハッシュのバイト列が変わっても緑になる（制約 (1) を spec が担保していない）
- 成果物: openspec/changes/st47-youtube-myactivity-split/specs/external-ingestion/spec.md（spec.md:207-210）/ tasks.md（Task 3.1・Task 4.1）
- 根拠: この Scenario の WHEN は「印を置いて格納した後に、解析器の版を上げ」る。試験の中では、印も後の照合も**同じ新しいコード**の `hash_shape` で計算されるので、`MyActivity` / `ChromeHistory` の同一性のバイト列が変わっても印と照合が一緒に動き、緑のまま。本人が ST12 の版で置いた 26 種類の印（design.md:168）が効かなくなる経路を、この Scenario は測っていない。測っているのは tasks.md:60 の `archive_shape_hash_is_unchanged_for_other_kinds`（文字列で書いた sha256 との一致）だけで、それは spec に無い（archive すると正典には残らない）。また tasks.md:60 は「`MyActivity`（製品「検索」）と `ChromeHistory` の形について `hash_shape` の結果が」と書いており、入力が手で組んだ形の JSON か、ファイルのバイト列から `classify` → `shape_for_file` → `hash_shape` を通したものかが決まっていない。`kind` の文字列は `shape_for_file` の `format!("{kind:?}")`（worker.rs:1190）で決まるので、手で組んだ形だと enum の名前を変えても落ちない
- kind: technical
- 提案: Scenario の WHEN を「ST12 の版の形（`{"kind":"MyActivity","products":["検索"]}` / `{"kind":"ChromeHistory","products":[]}` のバイト列の sha256）を直に入れた印だけがある状態で、同じ形のファイルを含む書庫を置く」に書き換える。Task 3.1 の試験はファイルのバイト列から分類・形・ハッシュまで通すと書く
- 処置: fixed 3.1 — Scenario の WHEN を ST12 の版の形のバイト列の sha256 を直に入れた印だけの状態に書き換え、tasks 3.1 の試験はファイルのバイト列から分類・形・ハッシュまで通すと書いた

## R4. D9 とコード: 確認待ちの表に版の列が無く、確認待ちの書庫の行は ON CONFLICT で形を更新しない —— 「旧い版の行」をどう見分けるか、移行が要るかが決まっていない
- 成果物: openspec/changes/st47-youtube-myactivity-split/design.md（D9）/ tasks.md（Task 4.1）/ proposal.md（Impact「移行: 1 本」）
- 根拠: `core.archive_pending_shape` の列は `user_id, sha256, inner_path, shape_hash, shape, file_sha256, made_copy`（migrations/202609181600_archive_ingestion.sql:117-126、202610042315_archive_pending_file.sql:11-12）で、解析器の版を持たない。tasks.md:65 の「`core.archive_pending_shape` の旧い版の行を新しい形の行に置き換える」は、どの行が旧い版かを表から引けない。さらに `record_pending_file` は `ON CONFLICT (user_id, sha256, inner_path) DO UPDATE SET file_sha256…, made_copy…` で `shape_hash` / `shape` を更新しない（worker.rs:1259-1261）。ダウンロードのフォルダに残っている確認待ちの書庫を、版を上げた後の走査が読み直しても、確認待ちの一覧の形は旧いまま残る。D9 は `read` の行の読み直しの穴（worker.rs:1694）しか挙げていない
- kind: technical
- 提案: D9 に置き換えの手段を書く（例: 版を上げた周に写しのある確認待ちの行をすべて取り直す／`ON CONFLICT` で `shape_hash` と `shape` も更新する／版の列を足す）。列を足すなら Task 1 か Task 4 に移行を書き、proposal の「移行: 1 本」を直す
- 処置: fixed D9 — 版は台帳の `pending_shape` の行の版で見分け（移行は足さない）、取り直しで `shape_hash` / `shape` を置き換え、`record_pending_file` の `ON CONFLICT` も更新する（D9 の 1、tasks 4.1）

## R5. D9 とコード: 確認待ちの書庫の読み直しは、表に積まれた形のハッシュで印を引き、読むときに新しい形を照らさない —— 旧い形の印が確認待ちの行を通す経路が Scenario に無い
- 成果物: openspec/changes/st47-youtube-myactivity-split/specs/external-ingestion/spec.md（spec.md:114 と Scenario spec.md:202-205）/ design.md（D9）
- 根拠: `confirmed_pending_copies` は `c.shape_hash = p.shape_hash`（確認待ちの表に積まれた形のハッシュ）で印を引き（worker.rs:1065-1088）、`ingest_confirmed_pending` は `reread_archive(..., false)` で、いまの形の印を照らさずに読む（worker.rs:1641）。版を上げた後も確認待ちの行に `f3e578ff…` が残っていて（R4）、そのハッシュに印がある（Q5 の「覚えていない」がそれに当たる。版の上げの直前に旧い形へ `--confirm` した場合も同じ）と、新しい分類器で読んだ YouTube のファイルが、新しい形（言語つき）に印が無いまま格納される。Q4 の「言語が当たらなければ確認待ち」も素通りする。要件本文 spec.md:114（旧い形の印を YouTube のファイルに効かせない）はこの経路も含むが、Scenario spec.md:202-205 は**新しく置いた書庫**の経路だけを測る。走査の輪の順（worker.rs:1820-1826。`reparse_older_versions` → `ingest_confirmed_pending` → 走査）のどこで形を取り直すかも D9 は決めていない
- kind: irreversible
- loss: rewrite-all
- 提案: Scenario を 1 本足す —— `旧い形の印があっても確認待ちの YouTube のファイルは版の上げの後に新しい形の印を待つ`（WHEN: 旧い版の確認待ちの行 `f3e578ff…` と、その形への印がある状態で版を上げて走査が来る / THEN: どの YouTube の論理ソースにも記録が増えず、一覧に新しい形で出る）。D9 に「確認待ちの読み直しも、読む直前にいまの形の印を照らす」か「取り直しを `ingest_confirmed_pending` より先にする」を書く
- 処置: escalated — deep.md の第 2 回 Q8（R5）。推奨（取り直しを先に回し、読む直前にいまの形の印を照らす）を design D9・spec・tasks 4.1 に書いてある

## R6. Migration Plan の「YouTube のファイルは新しい形で確認待ちに積まれる」は、読み終えた書庫ではコードがそうしない（黙って飛ばす）
- 成果物: openspec/changes/st47-youtube-myactivity-split/design.md（Migration Plan 2・D9）/ specs/external-ingestion/spec.md
- 根拠: design.md:224「読み手が次の走査で覚えている書庫を読み直し、YouTube のファイルは新しい形で確認待ちに積まれる」。`reparse_older_versions` は `reread_archive(..., true)` を呼び（worker.rs:1730）、印の無い形の写しは `continue` で飛ばすだけで、確認待ちの表にも台帳にも残さない（worker.rs:1493-1501）。他のファイルが読めればその書庫に版 2 の `read` の行が付き（worker.rs:1559-1576）、YouTube のファイルは二度と読み直しの対象にも一覧にも出ない。この状態になるのは旧い形 `f3e578ff…` に印があって YouTube のファイルが `read` で入った書庫で、Q5 の答え（覚えていない）がまさにそれ。そうした書庫の YouTube のファイルを新しい形で確認待ちに戻すか（戻して印を置けば、検索・その他が `c03-youtube-watch` の旧い行と並んで別の論理ソースに入る）、飛ばしたままにするかは、spec にも design にも Scenario にも無い
- kind: premise
- loss: rewrite-all
- 提案: Migration Plan 2 を、コードが実際にする動きに合わせて書き直すか、D9 に「読み終えた書庫の、印の無い形のファイルは確認待ちに積む」と書いて Task 4.1 と Scenario に足す。どちらを採るかで失われるものが変わるので、design.md:215 の「本人の運用の DB に同じ印が見つかったときだけ問う」の確かめ方（誰が・いつ・どの DB を見るか）も tasks に置く。いまは design の Risks と PR 本文（Task 7.4）にしか無い
- 処置: escalated — deep.md の第 2 回 Q8（R6）。推奨（読み直しで印の無いファイルは確認待ち）を design D9・Migration Plan・tasks 4.1 に書いてある。確認バッチの DB の確かめ方は design の Risks と PR 本文（7.4）に置いたまま（共有の docker volume を写す操作で、Task agent に持たせない）

## R7. 版を上げた後の確認待ちの読み直しは、台帳の名前・作られた時刻を引き継げない（ledger_meta が新しい版の `pending_shape` の行を探す）
- 成果物: openspec/changes/st47-youtube-myactivity-split/design.md（D9）/ tasks.md（Task 4.1）
- 根拠: `ingest_confirmed_pending` は `ledger_meta(pool, user_id, &archive_sha256, Some(super::PARSER_VERSION), "pending_shape")`（worker.rs:1633-1640）で元の読みの行を引く。版 1 で確認待ちになって専用のフォルダから「取り込み済み」へ移った書庫は、版 1 の `pending_shape` の行しか持たない（worker.rs:2188-2195。移った後は走査されない）。D9 で形を取り直しても版 2 の `pending_shape` の行を書かなければ、本人が印を置いた後の `read` の行は `file_name` も `created_at` も NULL、`inbox_kind` は既定の `inbox` になる（worker.rs:1449-1455）。これは final review R58 で一度直した型で、FR-16 の「それを運んだ書庫の作られた時刻も併せて保持する」が、本人の最初の YouTube の書庫で欠ける（台帳は追記のみなので後から埋められない）
- kind: technical
- 提案: D9 に「取り直しのときに版 2 の `pending_shape` の行を足す（1 回の読みに 1 行）」か「`ledger_meta` は版を問わずに `pending_shape` の行を引く」を書き、Task 4.1 の試験 `archive_youtube_reparse` に、読み直した `read` の行の `file_name` と `created_at` が元の読みの値であることを足す
- 処置: fixed D9 — 取り直しのときに、元の読みの名前・作られた時刻・置き場の種類を写したいまの版の `pending_shape` の行を 1 つ足す（D9 の 1）。Scenario `版の上げの後に印を置いて読み直した台帳の行は元の書庫の名前と作られた時刻を持つ` を足した（tasks 4.1）

## R8. D4（仮）の決まり文句の比べ方は、言語をまたいで評価に当たる（日本語の書き出しで題名が `Liked ` で始まる動画は視聴から外れる）。英語の表は本物で確かめた根拠が無い
- 成果物: openspec/changes/st47-youtube-myactivity-split/design.md（D4）
- 根拠: design.md:158「評価の判定は、項目の題名がどの言語の評価の決まり文句に当たるかで決める（ファイルの当たった言語には依らない）」と design.md:154（英語は先頭が `Liked ` / `Disliked `）を合わせると、日本語の書き出しで「Liked by the Sea を視聴しました」のような題名の視聴は英語の評価に当たって `c03-myactivity-youtube` に入り、凍結される。tasks.md:50 の単体試験は「動画名の途中に『を高く評価しました』を含む」だけで、この向きを見ない。英語の表の言い回しは deep.md:20-31 の本物の書庫（日本語の書き出し 1 冊）からは得られず、tasks.md:99-101 の realdata の確認も `languages: ["ja"]` しか見ない。英語の表が違っていると、英語の書き出しは視聴の決まり文句で `["en"]` になって確認待ちを通り、評価が視聴に凍結される（Q4 で本人が受け入れた「言い回しが変わる」とは別で、最初から確かめていない）
- kind: premise
- loss: rewrite-all
- 提案: 評価の判定は、そのファイルの当たった言語の表だけで行う（言語をまたがない）と D4 に書き、単体試験を 1 本足す。英語の表は「本物で確かめていない」と反転条件に書き、本人に返すか、英語だけは評価を外さずに `languages` を `["en"]` 以外の印（例: `en-unverified`）にして確認待ちに残すかを決める
- 処置: escalated — deep.md の第 2 回 Q9（R8）。推奨（当たった言語の表だけで見る。英語は印の前に評価の件数で確かめる）を design D4（仮）と tasks 2.2 に書いてある

## R9. D2（仮）の混ざったファイルでは、YouTube の項目が「その他」に入ることを本人が一覧で見られない
- 成果物: openspec/changes/st47-youtube-myactivity-split/design.md（D2・D7）
- 根拠: design.md:125-126 で、混ざったファイルは `MyActivity` として読み、YouTube の項目は視聴・検索に入らず `c03-myactivity-youtube` に入る。design.md:180 で行き先ごとの件数（`destinations`）を出すのは `YouTube` の種類だけなので、混ざったファイルの形の一覧には `products: ["YouTube", …]` しか出ず、印を置く本人は YouTube の視聴がその他に入ることを知らずに印を置く。反転条件（design.md:129）が成り立った後も、既に入った項目はその他に残る。本物の書庫に混ざったファイルが無い（design.md:127）ので起きる見込みは小さい
- kind: irreversible
- loss: rewrite-all
- 提案: 扉を開けたままにする既定で閉じられる —— `products` に「YouTube」を含む `MyActivity` の形にも `destinations` を出す（ハッシュは変えない）か、混ざったファイルを YouTube の種類と別の同一性にして必ず印を求める。どちらかを D2 に書き、Scenario `YouTube と他の製品が混ざったファイルは製品ごとのソースに入る` の WHEN に形の一覧での見え方を足す
- 処置: escalated — deep.md の第 2 回 Q10（R9）。推奨（その他へ入れ、一覧に件数）を design D2（仮）・D7・tasks 3.1 に書いてある

## R10. 観測できる振る舞いが design にしか無い（`destinations` の件数ではハッシュが変わらない／`details` が空の配列でも広告）
- 成果物: openspec/changes/st47-youtube-myactivity-split/design.md（D7・D3）/ specs/external-ingestion/spec.md
- 根拠: design.md:181「`hash_shape` は `destinations` を見ない（件数の変化では確認待ちにしない）」は ST12 第 3 回 Q12 の本人の決定に当たる振る舞いで、この Story で初めて形の JSON に書き出しのたびに変わる値（`destinations`）が入る。正典の Requirement（正典 spec.md:153）は件数を名指ししておらず、MODIFIED の spec.md:112 も件数に触れていない。`hash_shape` が形をまるごとハッシュにする実装でも、この change の Scenario はどれも落ちない。design.md:142「`details` が空の配列でも欄があれば広告に数える」も、受け付ける／数える振る舞い。spec.md:15（値を見ない）は Scenario を持たず、spec.md:38-46 の Scenario は `details` の値を決めていないので、値を見る実装でも通る
- kind: technical
- 提案: Scenario を足す —— `行き先ごとの件数が変わっただけでは YouTube のファイルは確認待ちにならない`（印を置いた形と件数だけが違う書庫を置くと格納される）、`値の違う details の欄を持つ項目も広告に数える`（英語の値・空の配列）
- 処置: fixed 3.1 — Scenario `行き先ごとの件数が変わっただけでは YouTube のファイルは確認待ちにならない`（3.1）と `値の違う details の欄を持つ項目も広告に数える`（2.2）を足し、spec の本文に「行き先ごとの件数」「空の配列も含む」を書いた

## R11. tasks の件数つき検証のうち 3 つは、既にある試験だけで下限を満たす（新しい試験が 0 本でも rc=0）
- 成果物: openspec/changes/st47-youtube-myactivity-split/tasks.md（Task 2.1・2.2・3.1）
- 根拠: `CT classify (≥2)`（tasks.md:41）は、既にある `archive_classify_prefers_json_shapes_and_accounts_for_html`（archive_tests.rs:511）・`archive_classify_skips_items_without_a_title_url`（archive_tests.rs:2046）・`archive::classify::tests::every_archive_logical_source_is_classified_as_location_or_item` の 3 本で満たされ、本文が求める `kind_of` の単体試験 4 本が無くても通る。`CT youtube (≥3)`（tasks.md:50）は既存の `archive_parse_youtube_separates_watch_and_decodes_search_query`（archive_tests.rs:699）と同じ Task の `archive_youtube_route` 12 本で満たされ、決まり文句の表の単体試験が無くても通る。`CT archive_shape_confirm (≥1)`（tasks.md:61）は既存の `archive_shape_confirm_*` / `archive_shape_confirmation_*`（archive_tests.rs:951 / 968 / 989 / 1031）で満たされる。しかも、足すはずの「`tools/archive-shape.sh` の出力に『京都』が無い」を、既存のどの cargo の試験に足すのかが決まっていない（これらの試験は道具を走らせない）
- kind: technical
- 提案: 新しい試験に専用の前置きを付けて絞る（例: `archive_youtube_kind_of (≥4)`・`archive_youtube_phrase (≥5)`・`archive_youtube_shape_listing (≥1)`）。道具の出力を見るなら smoke（Task 6.1）に置き、その `grep` を検証に書く
- 処置: fixed tasks.md — 絞り込みを新しい試験の前置き（`archive_youtube_kind_of (≥4)` / `archive_youtube_phrase (≥5)` / `archive_youtube_route (≥14)` / `archive_youtube_shape (≥8)` / `archive_youtube_reparse (≥5)`）に変え、道具の出力を見るものは smoke（3.2・6.1）に移した

## R12. Scenario のいくつかは、真偽を決める値か件数が書かれていない
- 成果物: openspec/changes/st47-youtube-myactivity-split/specs/external-ingestion/spec.md
- 根拠: spec.md:68-71「2 経路の同じ視聴」の THEN は「どちらの項目も `c03-youtube-watch` に格納され」で、原文が同じとき 1 行・違うとき 2 行（design.md:145）のどちらを期待するかが無い。spec.md:190 は言語を「日本語」と書くが、形の出力は言語の符号（design.md:153 の `ja`）なので、何の文字列を見るかが決まらない。spec.md:63-66 と spec.md:187-190 は 1 本に主張を 3 つ以上束ねている（片方だけ見る試験でも緑になる）
- kind: technical
- 提案: 2 経路の Scenario は原文の違う 2 件として「`c03-youtube-watch` に 2 件、他のソースに 0 件」と書く。言語は出力に出る文字列（`"ja"`）で書く。束ねた Scenario は行き先ごとに分けるか、THEN の各主張を試験が 1 つずつ見ると tasks に書く
- 処置: fixed 2.2 — 2 経路の Scenario を原文の違う 2 件で「`c03-youtube-watch` に 2 件、他のソースに 0 件」に、言語を `ja` に直した。束ねた THEN は試験が主張を 1 つずつ assert すると Global Constraints に書いた

## R13. smoke の照合を「弱めない」の検証が無い
- 成果物: openspec/changes/st47-youtube-myactivity-split/tasks.md（Task 6.1）
- 根拠: tasks.md:80「いまの照合を弱めない」に対し、検証は `tools/smoke.sh` rc=0 と手順書の `grep` だけ（tasks.md:82）。新しい `kind` の文字列 `YouTube` は、製品の名前 `["YouTube"]` にも同じ部分文字列があるので、`grep -q 'YouTube'` に置き換えると見分けた種類が違っていても通る。psql は jsonb を `{"kind": "YouTube", …}`（コロンの後に空白）で出すので、`"kind":"YouTube"` のような照合は空白の有無で落ちうる
- kind: technical
- 提案: 照合の文字列を tasks に書く（例: `grep -Eq '"kind": ?"YouTube"'`）。検証に `grep -q '"kind"' tools/smoke.sh` のような、照合が種類の欄を名指ししていることの確かめを足す

---

## 観点ごとの確かめた範囲

- **観点 1（deep → 正典）**: Q1・Q2・Q3・Q6・C2 は Scenario に写っている（spec.md:23-86）。Q4 は R1・R2・R8、C4 は R4〜R7、C6 は R5、Q5 の前提は R6 に書いた。「当初案を覆したもの」（ST12 D3）は正典の Scenario には元から無く（正典 spec.md:80-145 に 2 経路の記述は無い）、古い方が Scenario に残っていない。「要件へ戻すもの」は deep.md:97-101 で「なし」で、requirements.md に改訂は要らない。本人の決定に入っている数値（60 日）は Scenario spec.md:86 にある
- **観点 3（置き場）**: proposal の Capabilities（`external-ingestion` だけ）と `specs/` のディレクトリと INDEX.md:83 の表が一致し、前倒しは無い。spec に実装の名前（関数名・crate 名・列名）は無い（`{"kind":"YouTubeWatch",…}` は `tools/archive-shape.sh` の出力に出る値なので観測できる）。design に紛れた振る舞いは R1（D6）・R10（D7・D3）
- **観点 4（tasks）**: どの Task もコマンドと終了条件を持つ。検証の弱いものは R11・R13。依存の順（移行 → 分類と行き先 → ハッシュ → 版の上げ → 位置・最終日 → 道具）は正しく、鍵（ハッシュ）を変える Task 3 は、ハッシュに頼る Task 4 より前にある。人間の確認待ち（8.1）は「物理: realdata」と書いてある。Q5 の DB の確かめは R6
- **観点 5（Story）**: check_chain の観点 8 は通っている。ST47.md:12-14 の「価値」と 54-58 の「完了の判定」は deep の決定と矛盾しない（捨てる／捨てないの食い違いは無い）。ADDED の導出元 FR-14 / FR-16 / FR-17 は `satisfies` と一致する。MODIFIED の導出元 FR-22 / FR-23 は正典の Requirement からそのまま引き継いだもので、この Story が新しく満たそうとしているものではない
- **制約 (1)「YouTube 以外のハッシュを変えない」**: 確認待ちを要る種類は `YouTube*` / `MyActivity` / `ChromeHistory` だけ（worker.rs:7-15）なので、tasks.md:60 の 2 種類で足りる。spec の側の担保が無いことは R3
- **制約 (2) D9 とコード**: R4・R5・R6・R7
- **制約 (3) D2 / D4 / D6 の loss**: D6 → R1（conflict）、D4 → R8（premise）、D2 → R9（irreversible。扉を開けたままにする既定で閉じられる）
- 処置: fixed 6.1 — 照合を `grep -Eq '"kind": ?"YouTube"'` と名指しし、検証に `grep -qF` でその照合があることと `YouTubeWatch` が残っていないことを足した

