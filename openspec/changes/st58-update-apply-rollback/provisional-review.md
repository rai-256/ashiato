# ST58 仮で進めた決定の確かめ

## D10
- 判定: 否
- 根拠: 条件 1（失わない側が `uncaptured` を起こす）と条件 4（土台の決定が `rests_on` に無い）に当たる。
  D10 は裏の位置の権限が無い端末では `LocationService` を「起こさずに通知」にしている（design.md D10・defaults.md P1・tasks.md 9.1）。
  ところが裏の位置の権限は「あるとよい」権限で、断っても収集は始まる（`collector-android/app/src/main/kotlin/dev/ashiato/collector/MainActivity.kt:172-176`・`:409-411`）。
  そのため、この端末の数は少なくないとみられる。その端末では、更新の後、開くまでアプリ利用・生存信号・位置が取れない（`uncaptured`）。
  `LocationService` は位置が取れない間の種別 `specialUse` をもう持っている（`LocationService.kt:861-882`、`AndroidManifest.xml:114-123`）。
  その理由は ST06 の本人の決定 Q7「欠けたソースだけを止め、他は取り続ける」である（`LocationService.kt:866-868`）。
  受け手から `specialUse` の型で起こせば、位置のほかのソースは失わずに済む。手間は、種別を選ぶ関数に「裏から起こした」の引数を 1 つ足す程度で、範囲も要件も変えない。
  review/deep.md R11 の提案も「位置を除いた種別で起こす・起こさない」を書くよう求めていた。しかし D10 はそれより失う側（起こさない）を選び、理由を書いていない。
  `rests_on` は Q2 だけで、ST06 Q7 を挙げていない。
  なお、通知の権限も「あるとよい」なので（`MainActivity.kt:177-179`）、「通知で知らせる」が届かない端末も残る（D10 自身は「今と同じ」と書いている）。

## D25
- 判定: 確か
- 根拠: 条件 1: `ashiato_owner` は `CREATEDB` を持ち、DB の持ち主である（`tools/db-roles.sql:7`・`:11`）。
  DB 単位の `GRANT` / `REVOKE CONNECT` / `ALTER DATABASE … SET` は `migrations/`・`crates/server/src/grants.sql`・`tools/db-roles.sql` のどこにも無い（`grep -rn "CONNECT\|ALTER DATABASE"` の結果は `db-roles.sql:11` の持ち主の 1 件だけ）。
  そのため、`pg_dump` の custom 形式が持たない DB 単位の設定が戻しで欠けることは無い。
  `grants.sql:5-9` の付与は、所有者が自分の持ち物に対して行える。
  `initdb` は `scram-sha-256` で起こすので（`crates/installer/src/db.rs:12-21`）、既定の pg_hba で所有者は `postgres` にも `ashiato_restore` にも繋げる。
  所有者で足りないときは、名前の入れ替え（design.md D3 の 2）の前で戻しが落ちる。そのときも元の `ashiato` は壊れず、`uncaptured` / `discarded` / `exported` / `rewrite-all` のどれも起きない。
  条件 2: 手間は Task 6（tasks.md:78-88）の `戻しは DB の管理者の役割を使わない` の 1 段で、P20 を確かさ 4 に上げることも Task 6 に書いてある。
  条件 3: `follows` は無い。条件 4: 管理者の合言葉を捨てる形（`run_psql`）と `owner.json` の置き場（`crates/installer/src/main.rs:754-790`）は design の D3・D25 に書いてあり、ほかに挙がっていない土台は見つからなかった

## D26
- 判定: 否
- 根拠: 条件 1（失わない側が `exported` を起こす）と条件 4（土台の決定が `rests_on` に無い）に当たる。
  ST51 は「前の版」を、書いた順で最後の `applied` / `nothing_to_apply` の行の `to_version` から読む（`crates/server/src/update_history.rs:128-136`）。
  この値は、移行の試みと起動の行の `from_version`、起動の行を書くかどうかの判定に使われる（`update_history.rs:172-191`・`:280-290`）。
  D26 は、戻した DB に新しい版の `applied` / `nothing_to_apply` の行（`to_version` は落ちた新しい版）を書き直す。そのため戻した後は「前の版」が、動いていない新しい版になる。
  - 前の版が ST58 以後のとき: 戻した前の版のサーバが起きると `prev ≠ stamp` になり、`startup` の行（落ちた新しい版 → 前の版）が 1 行余計に足される。spec.md:484 の「最後の 3 行」の後に、決めていない 4 行目が付く
  - 前の版が ST58 より前（控えに書く回）: `update-history replay` は、次の更新が通った後に 2 行を表へ足す（design.md D18）。足された `applied` の行の `seq` は通った更新の行より後になるので、以後の試みと起動の行の `from_version` が、落ちた古い版になる
  更新の履歴は追記のみで（`migrations/202610300900_update_history.sql:19-33`）書き出しにも出る（specs/data-durability/spec.md:5）ので、誤った `from_version` の行は外に出たまま直せない（`exported`・`rewrite-all`）。
  Q6 の推奨（`failed` + `rolled_back`）は、`previous_version` が数えない値だけを書くので、この問題を起こさない。D26 は `follows` を持たず、Q6 と同じ向きであることを確かめていない。
  `rests_on` も Q6 だけで、ST51 の「前の版」の読み方（`previous_version`）を挙げていない。
  直す道は 2 つある。1 つは `previous_version` が `rolled_back` の行を数える（戻した先の版にする）こと。もう 1 つは、書き直す行に `applied` / `nothing_to_apply` 以外の値を使うこと。
  あわせて、`record-rollback` に渡す「当てた版の名前」をどこから取るかが design にも tasks（tasks.md:40）にも無い。`migrate` は当てた版の名前を出力しない（`crates/server/src/lib.rs` に `applied` の出力は無い）。新しい版の `MIGRATIONS` と戻した DB の版の記録の差、または `ashiato_failed_<時刻>` の行から導けるので、失われるものではない。ただし取り口を決める必要がある
