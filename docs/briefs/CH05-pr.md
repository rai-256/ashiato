# CH05 試験の DB が打ち切った run の残りで膨れないようにする（上流）

change: `openspec/changes/ch05-test-db-bloat/`。CH04 が立てる capability `test-database` に ADDED 3 つ（Scenario 24 本: 9・5・10）。
依頼の逐語・価値・壊してはいけないもの・完了の判定は `docs/changes/CH05.md`。**CH04 の上に積む**（Q1 の推奨）。`requires: [CH04]` は Q1 の答えの後に record が front matter へ書く。

> **第 1 回の深掘りの答えを待たずに、推奨の答えで先に書いた**（2026-10-20 の流れ）。下の Q1〜Q7 は**本人の答えではなく推奨**で、`deep.md` は `（答え待ち・推奨: …）` のまま。
> 答えが推奨と違えば `spec --answers` が仕様に写し直す。Q1 は前提の問い（premise）で、本人の答えが要る。
>
> 依頼の前提と実物が食い違っていた: 膨れの大半は殺された回の残りではなく、**正常に終わった回も**共有の `ashiato` に残した行で、その止め方（回ごとの database・次の回での片付け）は走っている CH04 が実装している（`premises.md`）。
> CH05 が持つのは、各 worktree に**いま積もっている** `ashiato` を機械が作り直す道と、殺した後の次の回を確かめる検査だけ。

## 深掘り（第 1 回。推奨で書いた）

| Q | 推奨（仕様はこれで書いた） | どこに効くか |
|---|---|---|
| Q1 CH05 の範囲（premise） | CH04 の上に積み、CH04 が持たないもの（膨れた `ashiato` の作り直し・殺した後の検査）だけをやる。`testdb.rs` は変えない | proposal、tasks の冒頭・1.1、`docs/changes/CH05.md` |
| Q2 いつ作り直すか | 検証の準備（`tools/verify-env.sh`）で大きさを測り、閾値（64 MiB。仮）を超えたら作り直す | Requirement 1、D1・D4、tasks 1.3・2 |
| Q3 何を見て見送るか | 回の助言ロック・回の作成の鍵・`ashiato` への接続のどれかがあれば見送る。消すときも接続を切らない | Requirement 2、D2、tasks 1.3 |
| Q4 作り直し方 | 中の `ashiato` database だけ（コンテナ・ネットワーク・volume はそのまま） | Requirement 1、D2 |
| Q5 作り直してよい作業ツリー | `-st<NN>`・`-ch<NN>`・`-up-*` だけ。main の checkout と `-verify` は測って出すだけ | Requirement 1、D3、tasks 1.2 |
| Q6 殺した後の確かめ方 | 量で見る（殺した回の database が残らない・`ashiato` の行数が同じ・大きさが 1 MiB を超えて増えない）。秒数は出すだけ。殺す時点は 2 つ | Requirement 3、D6、tasks 3・4 |
| Q7 ついでに直すか | どれも入れない。使われていない Docker の資源の片付けは別の change として申し送る | design Non-Goals |

## 仮決め（B）と反転条件（design の（仮））

- **D1（仮）** 閾値は `ashiato` の大きさ 64 MiB（ちょうどは作り直さない）。`TEST_DB_REBUILD_LIMIT_MB` で替える。反転条件: CH04 の後に縦串・開発用の起動だけで超える worktree が出たら上げる
- **D2（仮）** 1 つの psql の session（`postgres` database）で、直列化の鍵 4820252 を待たずに取り → 回の鍵 4820253 → クライアントの接続 → `FORCE` なしの `DROP`（`55006` は見送り）→ `CREATE`。反転条件: CH04 の鍵・名前が変われば合わせる。見送りが続く worktree が出たら知らせる形を足す
- **D3（仮）** 作業ツリーの種類は `tools/ports.sh` が出し、port の分岐もその種類で分ける（port の値は変えない）。反転条件: `-verify` が膨れて確認バッチが遅くなれば Q5 を本人に返す
- **D4（仮）** 検証の準備の順は「起動 → 待つ → 作り直しの判定 → 役割」。判定が失敗すれば検証の準備も落ちる（BLOCKED_INFRA）。反転条件: 判定が 1 秒以上かかれば大きさだけを先に読む
- **D5（仮）** 作り直しの self-test は本物の DB の使い捨ての `ch05_rebuild_probe_*` で確かめ、殺された self-test の残りは次の self-test が専用の鍵の下で消す。反転条件: 背景の psql の待ちで揺れれば印のファイルで同期する
- **D6（仮）** 殺した後の検査は `#[ignore]` の探りの試験を印で同期して SIGKILL し、`committed` の眠りの間に作り直しが「走っている回」で見送ることも見る。反転条件: CI でプロセスグループの kill が効かなければ pid を直接殺す。次の回の秒数が 10 倍を超える例が出たら Q6 を本人に返す

仮でない決定: D7（新しい試験の頭は `ch05_`）・D8（CI の rust job の 3 行と文書）。

## capability

`test-database` は CH04 の delta にしかなく正典に無い。CH05 は `## Purpose` を持たない ADDED で足し、CH04 の Requirement を MODIFIED しない（CH04 の本文を写さないので、CH04 の gate・fix に追従が要らない）。archive は CH04 の後。前倒しは無い。

## 下流の前提（要注意）

docker の既定のネットワークの割り当てが尽きていて、新しい compose の project は DB を立てられない（この上流の作業ツリーでも立たなかった）。下流の作業ツリーも新しい project なので、
このままでは DB を使う検証が BLOCKED_INFRA になる。直すのは使われていない資源の片付け（Q7 で別の change へ申し送り）か、人間がコンテナの無いネットワークを外すこと。Q7 の context に書き足してある。

## tasks・レビュー

- tasks: 6 Task・15 項目（作り直しの台本と種類 / 検証の準備への組み込み / 探りの試験 / 殺した後の検査 / CI と文書 / 全体の確かめ）
- spec-review: 19 件（`review/spec.md`）。escalated 1（R1 → 第 1 回の Q7 の context）・fixed 18（うち仮 9: D1・D2・D5・D6 ほか）。`review_triage.py` OK・`openspec validate --strict` OK・`evidence.py lint` OK
- questions の段の deep-review: 12 件（`review/deep.md`）

🤖 Generated with [Claude Code](https://claude.com/claude-code)
