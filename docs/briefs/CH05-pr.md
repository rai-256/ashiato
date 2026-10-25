# CH05 試験の DB が打ち切った run の残りで膨れないようにする（上流）

change: `openspec/changes/ch05-test-db-bloat/`。依頼の逐語・価値・壊してはいけないもの・完了の判定は `docs/changes/CH05.md`。
**CH04 と CH06 の上に積む**（`requires: [CH04, CH06]`）。依存の spec は gate 済みの SHA から読んだ（CH04 `9fe8c5e` PR #156・CH06 `c666673` PR #164。`deps.json`）。コードは取り込んでいない。

- `test-database`（CH04 が立てる）に ADDED 1 つ: 試験を途中で殺しても、次の回の後に残りが無いことを台本と CI が確かめる（Scenario 10 本）
- `verify-environment`（CH06 が立てる）に ADDED 1 つ: 作り直す検証の準備は、消えた作業ツリーのコンテナとネットワークを片付け、volume は消さない（Scenario 15 本）
- `verify-environment` の MODIFIED 1 つ: CH06 の「印があれば、検証の準備がこの worktree の DB を空から作り直す」に、片付けの例外の 1 文と Scenario 1 本を足す（本文と Scenario 5 本は c666673 から逐語）

## 深掘りで本人が決めたこと

| Q | 本人の答え | どこに効くか |
|---|---|---|
| Q1 CH05 の範囲 | CH04 の上に積み、CH04 が持たないものだけをやる（推奨と同じ） | proposal・tasks 1.1・`requires` |
| Q2〜Q5 作り直し・見送り・作り直し方・範囲 | 推奨と同じに答えたが、**Q8 で CH05 の範囲から外れた**（CH06 が検証のたびに空から作り直す） | design の Non-Goals |
| Q6 殺した後の確かめ方 | 量で見る（殺した回の database が残らない・`ashiato` の行数が同じ・大きさが 1 MiB を超えて増えない）。秒数は出すだけ。殺す時点は 2 つ（推奨と同じ） | test-database の Requirement・D2・tasks 4〜6 |
| Q7 ついでに直すか | **消えた作業ツリーのコンテナ・ネットワークを片付ける（volume は消さない）**（推奨と違う） | verify-environment の ADDED・D4・D5・tasks 2・3 |
| Q8 CH06 との重なり（第 2 回） | CH05 は殺した後の検査と片付けだけにし、作り直し・見送りは CH06 に任せる（推奨と同じ） | proposal・MODIFIED・`requires: [CH04, CH06]` |

## 仮決め（B）と反転条件（design の（仮））

- **D1（仮）** 置き場: 殺した後の検査は `test-database`、片付けは `verify-environment`（CH06 の Requirement 1 を同じ capability で MODIFIED）。MODIFIED には 3-way の base ができないので、tasks 8.1 が CH06 のいまの塊と `diff` で見る。反転条件: CH06 が Requirement 1 の名前・本文・Scenario を変えたら写し直す
- **D2（仮）** 殺した後の検査は `#[ignore]` の探りの試験を印で同期して SIGKILL（`insert` / `committed`）。反転条件: CI でプロセスグループの kill が効かなければ pid を直接殺す。次の回の秒数が 10 倍を超える例が出たら Q6 を本人に返す
- **D4（仮）** 片付けの台本 `tools/docker-tidy.sh`（`<main>-` の project・path が無く `git worktree list` にも無い・main と `-verify` を除く。繋がったコンテナは `docker ps -a --filter network=` で読む。`rm -f` に `-v` を付けない）。反転条件: 作業ツリーが残っているのに外された例が出たら `git worktree list` だけで見る
- **D5（仮）** 片付けは作り直す（`--plan` が `reset`）ときだけ、`reset_db` と `clear_blocking_network` の前に呼ぶ。落ちたら `片付けに失敗した` と rc の 1 行を出して続ける。反転条件: 印の無い経路で割り当てが尽きる例が出たら毎回呼ぶ形にし、CH06 の Requirement 2 も MODIFIED する

仮でない決定: D3（新しい試験の頭は `ch05_`）・D6（MODIFIED は CH06 の Requirement 1 の 1 文だけ。ほかの CH06 の Requirement は書き直さず、ADDED の本文で「何も消さずに」の範囲を名指す）・D7（CI の rust job の 4 行と文書）。
D 番号は Q8 の後に振り直した（旧 D6 → D2・旧 D7 → D3・旧 D9 → D4・D5・旧 D8 → D7。旧 D1〜D5 は Q8 で外した）。

## capability

前倒しは無い。`test-database` は CH04、`verify-environment` は CH06 の delta にしかなく正典に無い（`openspec validate` は MODIFIED の当たる先が無いことを情報として出す）。archive は CH04 と CH06 の後。

## tasks・レビュー

- tasks: 8 Task・14 項目（前提 / 片付けの台本 / 検証の準備への組み込み / 探りの試験 / 殺した後の検査の台本 / 本物で殺す / CI と文書 / 全体の確かめ）。CH05 で足した Scenario 26 本を全部名指す（写した 5 本の印は CH06 の検査のまま）
- spec-review（この段）: 9 件（R36〜R44。`review/spec.md`）。どれも technical で fixed（うち仮 4: D1・D4・D5）。観点 7 は CH06 の 4 件がどれも「前提」で、requires にある
- 前の段までの指摘 R1〜R35 は、番号の振り直しに合わせて処置の指す先を付け替えた（Q8 で対象が無くなった 8 件は `fixed design.md` として元の処置を残した）
- `openspec validate --strict` OK・`evidence.py lint` OK・`review_triage.py` OK・`spec_overlap.py check` OK・`check_chain.py` OK

🤖 Generated with [Claude Code](https://claude.com/claude-code)
