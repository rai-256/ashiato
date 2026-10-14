# ST47 code review（whole-branch）

## final review（e32f33a..72325ee）

席: final reviewer（`superpowers:requesting-code-review` の `code-reviewer.md`）。入力は review package
`.superpowers/sdd/tasks/review-e32f33a9..72325eef.diff`、Task 1〜6 の指摘の記録の Minor / ⚠️、`docs/handoff/ST47.md`。
判定: **No（直せば merge できる）**。D1・D3・D5〜D11 は design どおり、CT の本数は `cargo test -p ashiato-server -- --list` で全部下限以上。
申し送りの C4（FR-50 の削除の連鎖から漏れない）は満たしている（`stay_store` の `c03-myactivity-` 前置き + 見張り `every_archive_logical_source_is_classified_as_location_or_item` が 3 つの行き先を通す）。

## R1. deep.md Q9 の前提の訂正（日本語の評価は先頭一致）がコード・design・spec・試験の材料に入っていない
- 成果物: crates/server/src/archive/youtube.rs:58-62 / design.md D4 / specs/external-ingestion/spec.md の Scenario「高く評価した動画は視聴に入らない」/ archive_tests.rs:2197・archive_flow_tests.rs:4256/4575/4764/4786 の材料
- 根拠: deep.md:115-122（本人の補足「日本語の評価は先頭一致（高く評価しました  / 低く評価しました ）。本物で 196 件」と「spec の段が直すもの」）。youtube.rs の `ja` の表は `ends_with: true` で `rating: ["を高く評価しました", "を低く評価しました"]`。本物の書庫では評価が 1 件も当たらず、196 件が `c03-youtube-watch` に入って凍結される（言語は視聴・検索で `ja` と当たるので確認待ちにも戻らない）。上流の spec の段が訂正を当てず、下流は古い D4 に忠実に実装した（計画の欠陥）
- kind: technical
- 提案: `Phrases` に向きを決まり文句の組ごとに持たせ（ja の評価は先頭・視聴と検索は末尾、en は先頭）、ja の評価を `"高く評価しました "` / `"低く評価しました "` の先頭一致に。design D4 の表と前置きの文、spec の Scenario の題名、試験の材料を先頭の形に。PARSER_VERSION "2" は未リリースなので上げ直さない
- 処置: fixed D4 — 068cf849。ja の評価を先頭一致に（`youtube.rs` の向きを組ごとに）、D4・spec の Scenario の WHEN・試験の材料を先頭形に、`archive_youtube_phrase_japanese_rating_is_a_prefix_not_a_suffix` を足した。tasks.md 2.2 の本文（日本語は末尾）は凍結のため古いまま —— PR 本文に挙げる

## R2. 申し送りの D20 / R17（正典 external-ingestion の 60 日の文と格子の並び）が spec delta に入っていない
- 成果物: specs/external-ingestion/spec.md（delta）/ docs/handoff/ST47.md
- 根拠: 申し送りは 2026-10-13 13:18 に置かれ、ST47 の spec の段（同 15:35）より前。deep.md:12 の「`docs/handoff/ST47.md` は無い」は 08:35 時点の観測のまま。delta の MODIFIED は「印を置くまで格納しない」の 1 本だけ。ST14 は「ST47 が external-ingestion を触っている」ことを理由にこの直しを譲っている
- kind: technical
- 提案: 正典 `openspec/specs/external-ingestion/spec.md` の Requirement「取り込み器は自分の論理ソースに 1 日 1 回の生存信号を送る」（:588）と「稼働状況の画面に書庫のソースの格子と最終日と直近に置いた書庫が出る」（:676 / :697 / Scenario :709-712）を MODIFIED で delta に写し（Scenario は全部逐語）、60 日の文を「格子は 60 日、通知は最後の書庫から 75 日（`collection-coverage`）」に、並びを「Must の 5 ソース・集計・書庫のソース・退役したソース」に直す。文だけで振る舞いは変わらない
- 処置: fixed 7.5 — dc8ecc1e。正典の 2 Requirement を MODIFIED で Scenario ごと逐語に写し、60 日の文 2 か所と並びの文・Scenario の THEN だけを直した。並びの変化は `web/src/__tests__/archive-order.test.tsx` に集計を挟む場合を足して固定

## R3. `languages: []` の YouTube の形への印は、確認待ちの行が無いと `tools/archive-shape.sh --confirm` の断りをすり抜ける
- 成果物: tools/archive-shape.sh:36-52
- 根拠: 断りは `archive_pending_shape` にそのハッシュの行があるときだけ効く。ハッシュを一覧の外で知って先に印を置くと、表に無い言語のファイルが以後すべてその印で通る（deep.md 第 2 回 Q7 / design D6 が防ぐもの）。起きにくい（ハッシュを一覧の外で知る必要がある）
- kind: technical
- 提案: 格納側で、`languages` が空の YouTube の形は印があっても「印なし」と読む（`--confirm` の断りと二重にする）。それが大きければ断りの道具側で `languages: []` の YouTube の形のハッシュ（`hash_shape` が決定的に作る）を名指しで断る
- 処置: fixed D6 — 9f41b145。`worker.rs` の `is_file_shape_confirmed`（走査と `reread_archive`）と `confirmed_pending_copies` の SQL で、`languages` が空の YouTube の形は印があっても読まない。試験 `archive_youtube_shape_a_mark_on_an_unrecognized_language_does_not_store`。決まり文句を持たない合成の題名の材料を本物の形に揃えた

## R4. design の決定の一覧の D2 / D4 / D6 / D9 が「第 2 回 … で本人に問う中」のまま
- 成果物: design.md:26-36
- 根拠: 答えは deep.md に記録済み。PR 本文で D の（仮）を列挙するときに紛らわしい
- kind: technical
- 提案: 一覧の状態の欄を答えに合わせる（本人が決めたものは「第 2 回 Q<n> で本人が決めた」、仮のまま残るものは（仮）と反転条件）
- 処置: fixed design.md — 21177b53。一覧の状態の欄を第 2 回 Q7 / Q8 / Q9 / Q10 の答えに合わせた（D2 の製品の名前を持たないファイル・D4 の英語の表は仮のまま）

## fix wave（1f9685d..21177b5）の scoped re-review

席: re-reviewer（SDD の `re-review-prompt.md`）。R1〜R4 すべて ADDRESSED、fix の diff に新たな Critical / Important は無し。
fixer の申告: `cargo test -p ashiato-server` 835 passed / 0 failed（`drops_tests::drops_api_idempotent` が DB の deadlock で 1 回落ち、回し直して通過）、
`cargo clippy -p ashiato-server --all-targets -- -D warnings` rc=0。残り: tasks.md 2.2 の本文が古い（凍結のため触らない）。
