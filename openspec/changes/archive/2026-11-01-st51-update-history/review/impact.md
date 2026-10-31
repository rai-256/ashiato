# impact-review — st51-update-history

seen: IMP-01 medium, IMP-02 medium, IMP-03 low

## R1 — D1「全部の表に同じ付与」は、ST51 自身の D10 で崩れるのに holds が yes
- 対象: impact.dependencies[D1]（と tasks 2.x の検証）
- 種類: 依存の見落とし
- 指摘: D1 は `grants.sql:6` の「全表に SELECT/INSERT/UPDATE/DELETE」を根拠に `holds: yes` としている。しかし design D10 は、同じ `grants.sql` に `REVOKE UPDATE, DELETE ON core.update_history FROM ashiato_app` を足す。
  既存の `crates/server/src/app_role_tests.rs:37-62`（`app_role_privileges_reach_every_table`）は `core` の全表（`pg_tables` から引く）について app 役割の UPDATE・DELETE が true であることを assert する。update_history は UPDATE・DELETE が false になるので、この試験は落ちる。
  tasks.md:52 は「既存のアプリの役割の試験が通る（`cargo test app_role_tests`）」を検証に置いており、このままでは通らない。依存（読む側 = 全表を同じ付与と前提する試験）が `dependencies` に無い。
- 直し方の案: 「全表に同じ付与を前提に assert する app_role_tests」を依存に足して `holds: no` とし、試験に update_history の例外（UPDATE/DELETE は false）を足す作業を tasks に置く。
- 処置: fixed: 依存 D17（app_role_tests の全表 UPDATE・DELETE の前提。holds: no）と根拠 E17 を足し、IMP-05（low）・open（未解決）・受け入れ条件に「spec の段で tasks 1.2 に試験の直しを足す」を置いた。reading は仕様を書き換えないので tasks は直していない

## R2 — D3/S3 の「落ちた回の行が消える」は、IMP に上がらず、閉じ方も書かれていない
- 対象: impact.dependencies[D3] / scenarios[S3] / findings
- 種類: 閉じ方
- 指摘: D3 は `holds: no` で S3 に繋がるが、IMP が無い。S3 の `decision` は「深掘り Q2 の答え」に預けている。ST58 より前に手で戻すと落ちた試みが記録から消える（design.md:132、docs/handoff/ST58.md の R1）。
  FR-119 の戻しはいまは手作業で、「落ちた回」はまさに「更新で止めた／壊れていた」の区別が要る回である。
  Q2 が仮の答えである間は IMP の `if_deferred.interim` に当たる「ST58 まで手で戻す前に何を控えておくか」（例: migrate の標準エラーの控え）が書かれていない。S1 の IMP-01 とは別の、同種の損失である。
- 直し方の案: S3 を IMP（low〜medium）として立て、ST58 までの間に残す手当て（戻す前に落ちた版・SQLSTATE を控える手順など）を interim に書く。手当てが無いなら unknowns に書く。
- 処置: fixed: IMP-04（medium。ST58 より前に退避から戻すと落ちた回の行が消える）を立て、D3・S3 を繋ぎ、undefined に「戻す前に控える手順は仕様に無い」、unknowns と options を書いた。high ではないので interim は求められないが、受け入れ案に「標準エラーで読む」を置いた

## R3 — IMP-01 の option 1「ST58 の知らせが標準エラーの報告を読む」は予定にとどまる
- 対象: impact.findings[IMP-01].options[0]
- 種類: 閉じ方
- 指摘: 受け入れの根拠は ST58 が標準エラーを読むこと。docs/handoff/ST58.md は標準エラーの形（`error: kind=…` など）を伝えているが、それは終了コード 3 の回に「戻さない」ための注意で、止まった試みを記録として残す役目は書かれていない。FR-120 の知らせは「戻したとき」の知らせ（requirements.md:1016）であり、戻さない終了コード 3 の回に出るとは限らない。
  ST58 が読むことは確かめられておらず、`feasibility: unverified` のまま。
- 直し方の案: ST58 の handoff に「終了コード 3 の回の標準エラーを残す／知らせる」を足すか、option 1 の根拠から ST58 を外す。
- 処置: fixed: IMP-01 の受け入れ案から ST58 への依存を外し、「止まった理由は migrate の標準エラーの報告で、本人がその場で読む」にした

## R4 — situations.normal / update の `can` は、Scenario の写しで、実運用の経路を通していない
- 対象: impact.situations[update].can, situations[rollback].can
- 種類: 言い切り
- 指摘: 「サーバを入れ替えて移行・起動すると 1 行残る」「前の組み立てに戻すと『新 → 前』の 1 行が残る」は spec の Scenario の言い換え。stack.sh（E4）の呼び方（migrate → 起動）で本物の bin を通した確認が `evidence` に無い。
  特に rollback は、前の組み立てが DB の版を全部知っている場合に限る（知らなければ IMP-01）。`can` にその限定が `cannot` 側にしか無く、`can` 単独では広く読める。
- 直し方の案: `can` に「前の組み立てが DB の版を全部知っているとき」を書き、verify に smoke（tools/smoke.d）で通すことを置く。
- 処置: fixed: situations の update.can に「D-01 の版を全部知っていれば」と「本物の bin で stack.sh の流れを通した確認はまだ無い」を、rollback.can に「前の組み立てが D-01 の版を全部知っているときだけ」を書いた

## R5 — IMP-02 の「見つけていない」は、探した範囲が狭い
- 対象: impact.findings[IMP-02].unknowns
- 種類: 根拠
- 指摘: 確かめたのは INDEX で FR-121 を持つ Story だけ（INDEX.md:74）。requirements.md:1643 の扉 #14（欠損の意味）は FR-121 を関与要件に加え、更新の履歴が「更新で止めた」を区別する担い手と書く。その扉を満たす格子側の Story（FR-80・FR-85 など）が更新の履歴を画面で併記するかは、`unknowns` が「確かめていない」とするだけで、探した範囲が INDEX の FR-121 検索に限られる。medium の根拠としては足りるが、`verify` を実際に回してからでないと basis は `unconfirmed` のまま。指摘というより、第 2 回までに verify を実行する材料。
- 直し方の案: 扉 #14 の関与要件を持つ Story を `docs/stories/stories.json` で引いて、画面に出す Story の有無を書く。
- 処置: fixed: 扉 #14 に触れる Story（ST01・ST02・ST04・ST06・ST07・ST14・ST15）を引き、ST51 以外は archive 済みで更新の履歴を見せる振る舞いを持たないことを IMP-02 の unknowns に書いた。basis は unconfirmed のまま（未 Story の要件は見ていない）

