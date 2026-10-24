# CH01 の前提

外の形式に依る決定なし（9 件はどれもこの repo のコードと道具の中の話）。

下は、依頼の 4〜9（未確認）を questions の段で main（b0865d36）のコードに当てて確かめた結果。直す前に下流がテストで再現する（確かさ 4 へ上げる）。

| 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|
| 1. ST17 の錠の節が `tools/check-immutable.sh` の本体にある | 1 の移し替え | 本体を読む | 有り（1174〜1343 行の節と、1367 行の「戻し手順を逆順に当てる」の中の ST17 の段）。ST03・ST04・ST19・ST22・ST33・ST12・ST21 の節も本体にある（`*.d/` の取り決め 2026-10-15 より前に入ったもの） | `tools/check-immutable.sh:1174` `:1367` | 2 |
| 1''. `*.d/` の節は、全 Story の版を戻して schema を作り直した後に読まれる | 1 の移し方 | 台本を読む（review/deep.md R1） | そのとおり（読む口は戻しの後。主観・場所・主張などの版は戻した状態で、当て直すのは ST03 の 5 本だけ）。`*.d/` の節はまだ 1 本も無い。ST17 の節は ST19 の節の `$AU` と `immutable-check` の行に依る | `tools/check-immutable.sh:1352-1374` `:1444-1447` `:1507` | 2 |
| 1'. `check-shared-appends.sh` は本体の途中への新しい節の挿入を通す | 1 のすり抜けの検査 | 台本を読む | 通す（末尾＝最後の空でない行より後ろに触れた hunk だけを落とす。途中の既存の節を直すのを通すための意図した形） | `tools/check-shared-appends.sh` `tail_touched()` | 2 |
| 2. 戻しの検査が NG の後にも OK を出す | 2 | 台本を読む | 出す（`|| { NG; fail=1; }` の次の行が無条件の `echo "  OK feeling …"`） | `tools/check-immutable.sh:1331-1336` | 3 |
| 3. トークンを追跡ファイルに足して、trap なしで戻す | 3 | 台本を読む | そのとおり（`printf … >>collector-android/gradle.properties` → gradle → `git checkout`。subshell が止められると戻らない。`record-run` は TERM の後に KILL を送り、KILL では trap も走らない（review/deep.md R7））。`collector-android/gradle.properties` は追跡されている | `tools/record-env.sh:166-172` | 3 |
| 4. 本文を消した滞在（`payload='{}'`）は終わりが無く、区間が始まりの一点に潰れる | 4 の直し方 | `target_state_error` を辿る | 潰れる（`end` が無いと `stay_end = stay_start`。記録の始まりが滞在の始まりより後なら `start > stay_end` で `invalid_feeling_target`）。いまのテスト `st17_reject_target_deleted_stay_is_accepted` は**削除の印**だけで本文は残しており、記録の始まり＝滞在の始まりなので当たらない | `crates/server/src/feelings.rs:294-303`、`feelings_reject_tests.rs:464`、消去の形は `migrations/202609120944_gates.sql:188`。**ただし製品のコードに滞在の本文を消す経路は無い**（消すのは印だけ、作り直しは同じ id の本文を書き換える。review/deep.md R3）。終わりの無い滞在を始まりの一点とみなすのは ST16 の `span_of` と同じ | 3 |
| 5. 滞在の行は押した値を送る前に印にし、断られても届かなくても戻さない | 5 | 画面のコードを読む | そのとおり（`setStayPicks` が `post` より前、`post` の断られた・届かなかったの枝は `stayPicks` に触れない）。**日の欄（`pick`）も断られたときに戻さない**のは同じ | `web/src/FeelView.tsx:303-307` `:256-276` | 3 |
| 6. 前/後は画面が読み込んだときの `now`、サーバは記入の日時で決める | 6 | 両方を読む | そのとおり（画面: `setNow` は読み出しのときだけ、`phaseOf(dayTarget, now)`。サーバ: `phase_of(written_at, day_end)`。記入の日時は押したときの `new Date()`） | `web/src/FeelView.tsx:98,122,222`、`crates/server/src/feelings.rs:438` | 3 |
| 7. 取り込みの「人が書いた記録」の規則の写しが 3 つある | 7 | 読む | 3 つ（主張 1023〜1047・主観 1059〜1077・場所 1091〜1104）。**主観の tz が +540 固定なのは spec の決定**（`subjective-log` の「記録の地域が東京でない主観は受け付けない」）で、食い違いではない。順も違う（主観は tz を形と一緒に先に見る） | `crates/server/src/lib.rs:1023-1104`、`openspec/specs/subjective-log/spec.md` | 2 |
| 8. 同じ日を 2 回取りに行く | 8 | 読む | 2 回（その日だけと、その日で終わる 30 日）。押して受理されると `tick` で両方が読み直す。分けている理由はコメントに「片方の失敗が他方を巻き込まない」 | `web/src/FeelView.tsx:112-149` | 3 |
| 9. 始まりの日を求める `min(event_time)` は走査が重い | 9 | 移行の索引を読み、合成データで EXPLAIN ANALYZE（review/deep.md R2） | **重くない**。`event_by_source_time (logical_source, event_time)` の索引に乗る。使い捨ての postgres:17 に位置 100 万行・主観 500 行で 4 ページ・0.079 ms。再現しないので直さない | `migrations/202609112113_source_lifecycle.sql:71`、`feelings.rs:624` | 4 |
