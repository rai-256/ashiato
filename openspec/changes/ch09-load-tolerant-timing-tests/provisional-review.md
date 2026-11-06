## D18
- 判定: 否
- 根拠: 1 の条件（守れていないものを `keeps` が「守れる」と書いている）で否。`keeps` の「60/30 の上限の行は archive 後に check_agreed_tests が守る」は誤り。
  `check_agreed_tests.py` の `spans()`（`scripts/check_agreed_tests.py` の `ASSERT.search(ln)`）は、期待の正規表現に当たった**その 1 行だけ**を守る。
  `crates/server/src/testdb_tests.rs:381-385` の比べは複数行の `assert!(` で、上限 `std::time::Duration::from_secs(30)` は 382 行目にあり、守られる行には入らない。
  実行して確かめた: `python3 -I -c '…m.spans(open("crates/server/src/testdb_tests.rs").read())…'` → Scenario「上限を超える問い合わせは、止まって失敗する」で守られる行は
  `(381, 'assert!(')` と `(386, 'assert_eq!(code.as_deref(), Some("57014"), …)')` の 2 つだけ。`from_secs(30)` を `from_secs(5)` にしても `assert!(` の行は HEAD に残るので通る。
  したがって archive の後も機械が捕まえないのは相手の `pg_sleep(60)` だけでなく上限の 30 秒もで、`cost` の「相手の `pg_sleep(60)` を短くする書き換えは…捕まらない」は残るものを少なく書いている（design.md:270 の「上限の比べの行（期待の行）を守る」も同じく誤り）。
  2,000 件の側は確か: `runtime.rs:3678` の `assert_eq!(QUEUE_PER_TICK, 2_000, …)` は 1 行で、同じ `spans()` が Scenario「読みを放した後、見回り 1 回で積む履歴の記録は 2,000 件以下」の期待の行として拾う。
  失うもの（`uncaptured` / `discarded` / `exported` / `rewrite-all`）はどちらの側にも無い（試験の字面だけで、データに触れない）。`cost` は高くつかず・範囲を越えず・要件の本文を変えない。`follows` は null。
  ただし `rests_on` が空で、60 秒・30 秒の値そのものは design D6（仮。Q3 の答えの「例: 相手 60 秒・上限 30 秒」）に載っている。D6 が変われば D18 の固定する対象も変わるので、`rests_on` に D6 が要る。
  人間に返すときは、「打ち切りの 60/30 は相手も上限も archive 後に機械で守られない（上限の行を 1 行にまとめるか、design D18 の反転条件の `check-inventory.sh` の検査を足せば守れる）」を正しい `keeps` / `cost` として示す。
