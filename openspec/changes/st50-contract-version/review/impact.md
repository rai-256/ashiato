# impact-review — st50-contract-version

seen: IMP-01 medium, IMP-02 medium, IMP-03 medium, IMP-04 medium

## R1 — 確かめる口が 404 を返さない S-01（V-01 を S-01 が配る形）の扱いが依存に無い
- 対象: impact.dependencies（T2 / T4 の読み手）、situations[update].cannot、rules[R4]
- 種類: 依存の見落とし
- 指摘: 分析は「ST50 より前の S-01 は GET /contract に 404 を返す」を前提に、止まる・更新を促す、を組んでいる（design D9・D11。根拠は premises P2「router に fallback が無い」）。
  一方 docs/stories/ST52.md:21（FR-104 の★）は「V-01 は S-01 が配る」と決めている。S-01 が画面の静的ファイルを配る形にすると、知らない道に画面の入口（200 の HTML）を返す fallback を持ちうる。
  そのとき ST50 より前の S-01 は /contract に 200 の HTML を返し、収集アプリは D22 の経路（送らずに残す・更新は促さない）に入る。
  つまり「古い S-01 に新しい収集アプリを繋ぐと『サーバを更新してください』が出る」（S3・overview・IMP-03 の「更新の知らせが並ぶ」）が、その組み合わせでは出ず、記録だけが黙って溜まり携帯は 90 日で捨てる。
  ST52 は requires に ST50 を持たない（ST52 の front-matter は ST48・ST49）ので、ST52 が先に入る並びがありうる。dependencies に ST52 の画面配信と 200 の読めない答えの組み合わせが無い。
- 直し方の案: dependencies に「画面を S-01 が配る形（ST52）で知らない道が 200 を返す」を unknown で足し、D22 の反転条件に「ST52 の配信が入ったら 200 の HTML も促す側へ」を結ぶ（または ST52 側の申し送りにする）。

- 処置: fixed: impact.dependencies に D16（画面を S-01 が配る形の古い S-01。holds unknown・basis guess・根拠 E22）を足し、IMP-05（medium・guess。ST52 への申し送り・D22 の反転・受け入れの 3 案）として findings と scope.residual に出した

## R2 — D13（途絶の知らせの前提）の根拠が主張の意味を支えていない
- 対象: impact.dependencies[D13].evidence（E14）
- 種類: 根拠
- 指摘: E14 の quote は OutagePoller.kt:12 の「GET /outages?device_id= の 1 回の問い合わせ」という説明の 1 行だけで、これは「携帯が途絶を問う」ことしか示さない。
  D13 の主張「S-01 が途絶と言う期間は収集が止まっていた期間という前提」と、「版で止まると生存信号が届かず収集中でも途絶と出る」は、S-01 側の途絶の算出（生存信号と記録の到着から作る側。crates/server の coverage*）を読まないと言えない。
  holds: no の結論自体は design D3 から論理で言えるが、evidence が指す場所は結論の根にならない。
- 直し方の案: evidence に S-01 が途絶を作る箇所（coverage* の途絶の算出）を足すか、E14 を外して basis: derived の根拠を D3 の決定に置く。

- 処置: fixed: impact.dependencies[D13] の evidence に S-01 の途絶の判定（crates/server/src/outage.rs:133。記録と生存信号の到着で生きているとみなす）を E23 として足した

## R3 — IMP-02 の unknowns「ST58 の自動の戻しが契約の版をまたぐか」は、いま分かる材料がある
- 対象: impact.findings[IMP-02].unknowns / options[1]、dependencies[D15]（E16）
- 種類: 根拠
- 指摘: E16 は ST58 の表題 1 行だけ。ST58 に載っている FR-119（docs/stories/ST58.md の FR-119・Q9）は「更新の間 S-01 は止まっていて」「移行が失敗したら戻す」で、
  自動の戻しは移行が失敗した更新の途中（新しい S-01 がまだ収集アプリを受けていない間）に起きる。そのため自動の戻しで「戻した S-01 より新しい版の行が溜まる」は起きにくい。
  実際に起きるのは、更新が成功して新しい S-01 が動いた後に手で前の組み立てへ戻す場合（S2 の initial）。分析は 2 つを区別せず、自動の戻しも同じ帰結の入口として unknown に置いている。
  帰結の重さは変わらないが、「ST58 の上流で条件にする」の option は FR-119 を読めば要らないと分かる可能性が高い。
- 直し方の案: unknowns を「手で戻す場合だけが入口か」に絞り、FR-119 を evidence に足す（ST58 の自動の戻しは移行失敗の途中なので入口でない、と derived で書く）。

- 処置: fixed: IMP-02 の unknowns を「手で戻す場合だけが入口か」に絞り、FR-119（E24）を根拠に足した。D15 の who を手で戻す操作にし、ST58 へ条件を付ける案を「戻す順の節に書く」案に替えた。W2 の例外の文も直した

## R4 — IMP-01 の verify の手順が、書かれた設計では「通ってしまう」形にならない
- 対象: impact.findings[IMP-01].verify
- 種類: 根拠
- 指摘: verify は「写しの android.ingest にだけ欄を足した木で check-contract-version.sh を走らせ、通ってしまうことを確かめる」。
  だが design D7 の 1 は、収集アプリ側のずれを St50EnvelopeShapeTest / st50_envelope_shape（実際の直列化と写しの比較）が android / collector-windows の job で落とす、と言う。
  写しだけを手で書き換えた木は、check-contract-version.sh は通るが単体試験が落ちるので、IMP-01 の「穴」を示せていない。
  穴が開くのは、収集アプリに欄を足して `--write` で写しを作り直し、版を上げ、S-01 の型だけ足し忘れた木（写しの android 側に欄があり s01 側に無い）で、これを検査が見ないこと。
- 直し方の案: verify を「`--write` で android 側に欄を足した写しを作り、s01 側を足さずに版を上げた木で通ること」に書き換える。

- 処置: fixed: IMP-01 の verify を「--write で android 側にだけ欄のある写しを作り、版と履歴を上げ、S-01 の型に足さない木で検査と単体試験が通ってしまう」に書き換えた

## R5 — overview の「PC は捨てない」が仮の決定に頼っているのに限定が落ちている
- 対象: impact.overview
- 種類: 言い切り
- 指摘: 「携帯では 90 日・2 GB の上限を超えた分は捨てられる（PC は捨てない）」。PC に上限を置かないのは design D13（仮）で、反転条件つき（FR-125 の★の読み。NFR-7 の読み）。
  確定した効果として書いてあるが、実際は仮の決定の帰結で、PC のディスクが尽きたときの扱いは見ていない（残る・溜め続けるだけで、尽きたあとは分析が見ていない）。
- 直し方の案: 「PC は上限を置かない（D13 仮）」と限定を付け、ディスクが尽きる場合は not_examined に足す。
- 処置: fixed: overview を「PC の未送信には上限を置かない（仮の決定 D13。PC のディスクが尽きた場合は見ていない）」にし、ディスクが尽きる場合を coverage.not_examined に足した
