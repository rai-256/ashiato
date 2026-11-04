# CH09 の深掘りの問いの独立検査（deep-review）

対象: `openspec/changes/ch09-load-tolerant-timing-tests/deep-questions.json`（Q1〜Q7）・`premises.md`・`docs/changes/CH09.md`。
手順 1〜8 は問いの一覧を見る前に、依頼と既存コードからやり直した。その後で一覧と突き合わせた。

## 手順ごとの結果

- 手順 1（要件どうしの衝突）: change なので `satisfies` は空。依頼の中では、「2 の形 = 上限を延ばす」と「`cargo test --workspace` が負荷の下で通る」が、2 の試験については両立しない見込みがある → R1。もう 1 つ、「製品のコードは変えない」と「1 の試験で Saving を待つ退行も捕まえる」「2 の試験の競り合いを除く」がぶつかる → R2・R3
- 手順 2（扉の幅）: 「製品のコードは変えない」が、振る舞いを変えない試験用の口（`#[cfg(test)]`）を許すかどうかで読みが分かれる。Q1・Q3 がそれぞれ別に触れていて、範囲の問いとしては 1 回も問われていない → R3
- 手順 3（新たに立つ一方通行）: 該当なし（確かめた範囲: 7 問の全選択肢。変わるのは試験・`tools/` の台本・PR 本文だけ。捨てる・外に出す・凍結した行を書き直すものは無い。7 問とも `kind: open`・`loss` 無しで合っている）
- 手順 4（日常に影響する選択）: 上限を 30 秒にすると、本当に落ちる試験は 1 本ごとに 30 秒待ってから落ちる。画面の単体試験は暗黙の 1 秒の待ちが 317 か所ある → R6
- 手順 5（既存コードが依頼を満たしていない箇所）: 2 の試験の落ち方は、上限の短さではなく、止めた時計を進めるのが早すぎる競り合いと読める。模型を走らせると、上限を 30 秒にしても同じ所で止まった → R1。premises の「Saving は製品のコードを変えないと止められない」は、実物の `atomic_write` で崩れる → R2
- 手順 6（過去の取りこぼし）: 繰り返し出た族「検証コマンドが成立しない（11 件）」と観点「凍結前に検証コマンドを空走させる」「合成負荷の実測でパラメータを変えると結論が反転しないか」が当てはまる。完了の判定の 3 つのうち問われているのは負荷のかけ方だけ → R7
- 手順 7（族の網羅）: Q1・Q3・Q4 の族「実時間で性質を確かめている試験」に、打ち切りが効くことを時間で見る型・「起きない」を短い窓で見る型・ライブラリの既定の暗黙の上限が載っていない → R4
- 手順 8（前提の確かさ）: `loss` を持つ決定は無い（7 問とも open）ので、形式上の「前提が誤りうる」には当たらない。ただし premises の 3 行目（確かさ 2・走らせていない）が 2 の試験の直し方の土台になっていて、走らせると誤りだった → R1。4 行目は Rust だけを数えていて、Q2 の範囲を決める規模が欠けている → R5。外の形式に依る決定は無く、premises.md は在る

---

## R1. 2 の試験の落ち方は「上限が短い」ではない。上限を 30 秒に延ばしても落ちる道が残る（依頼の 2 の前提と premises 3 行目の誤り）
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/deep-questions.json（Q4・Q5）・premises.md（3 行目）
- 根拠:
  - `crates/server/src/archive_scan_interval_tests.rs:255-259`: 2 歩目以降は、`wait_for_scans` が数の増えたのを見た**直後に** `advance(1001ms)` し、その後に固定の `settle(25)` を置いている。取り込み器が眠りに入ったかどうかは確かめていない
  - `crates/server/src/archive/worker.rs:1544-1571`: 走査の回数は `INSERT … RETURNING` で commit された時点で試験から見える。その後に取り込み器は、今日の生存信号の有無を引く往復をもう 1 回する（`exists`）。`wait_for_next_scan` に入るのはその後
  - `crates/server/src/archive/worker.rs:2226-2249`: `waited_from` は `wait_for_next_scan` に入った時点の（止めた）時計で決まり、その後に `sleep(1s)` する。時計を進めた後で `waited_from` が決まると、1 秒の眠りの期限は進めた後の時刻から数えられる。もう誰も時計を進めない
  - `archive_scan_interval_tests.rs:84-85`・`:149`: 待っている間は blocking の task がいつもあるので、止めた時計は勝手には進まない（試験の作りとしてそうしてある）
  - `crates/server/src/scheduled_tests.rs:112-113` のコメントに同じ型が書いてある:「刻みが粗いと、取得の後の読み直しの間に時計が先へ進み、眠りの始まりがずれて回数が減る」
  - 模型を走らせた（`/tmp/ch09-exp`。tokio 1.53.1、current_thread、止めた時計。上の順をそのまま写し、回数の後の往復を実時間 N ms の I/O にした）:
    `post=1ms bound=5s -> Ok`・`post=40ms bound=5s -> Err("step 2: want 3 got 2")（5.6s）`・`post=40ms bound=30s -> Err("step 2: want 3 got 2")（30.8s）`。
    回数の後の往復が `settle(25)` より長いと、その歩でずっと止まる。上限を延ばしても、落ちるまでの時間が延びるだけだった
  - 実際の落ち方 `走査が 4 回に届かない: 3`（CH09.md）は、3 歩目で数が止まったまま 5 秒経ったもので、この型と合う。load 9〜14 / 16 コアで走査 1 回に 5 秒かかるとは考えにくい
  - premises.md の 3 行目が確かめたのは「延ばしても普段の所要時間が変わらない」ことだけで、「延ばせば落ちなくなるか」は確かめていない（確かさ 2・走らせていない）
- kind: premise
- 提案: 第 1 回に問いを足す。「2 の試験は、上限を延ばすだけでは落ちる道が残る。何で直すか」。選択肢の候補は 3 つ。(a) 進める前に、取り込み器が眠りに入ったことを観測する（観測の口が要るので R3 にかかる）。(b) 届くまで 1,001ms ずつ進め直す（「仮想時間は 10 秒ちょうど」の assert が弱まる）。(c) 進める前の固定の眠りを延ばす（遅くなるうえ、競り合いは残る）。Q4 の推奨「観測できる条件が無ければ残して一覧に載せる」をそのまま採ると、2 の試験は直らないまま一覧に載る。だから Q4 の why にもこの型を書く。premises には「延ばせば落ちなくなるか」の行を足し、走らせて確かめる
- 処置: fixed deep-questions.json Q8（2 の試験の直し方を足した。推奨は届くまで進め直す）・Q4 の why・premises.md 3 行目

## R2. Q1 の context「Saving を待つ退行は、製品のコードに差し込み口を足さないと止められない」は実物と合わない
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/deep-questions.json（Q1）・premises.md（2 行目）
- 根拠:
  - `crates/collector-windows/src/history/collect.rs:542-555`: `Plan::save` は帳面と `last_success.json` を `crate::fsutil::atomic_write` で書く
  - `crates/collector-windows/src/fsutil.rs:17-22`: `atomic_write` は `path.with_extension("tmp")` を `File::create` で開いてから `sync_all` する
  - 試した（python で同じ開き方。`O_WRONLY|O_CREAT|O_TRUNC`）: 名前付きパイプ（FIFO）をその `.tmp` の場所に置くと、読み手が開くまで開く側が止まった（`blocked after 1s: True`）。放した後の `fsync` は `EINVAL` で失敗した
  - つまり Linux の上では、製品のコードを変えずに、試験だけで Saving の別スレッドを止められる。代わりに、その取得は「帳面を書けない」の失敗で終わる（`sync_all` が失敗するため）
- kind: premise
- 提案: Q1 の context と premises の 2 行目を直す。「止められるが、その取得は失敗で終わる。Windows の上では使えない」とする。Q1 に選択肢を足す。「止めた読み ＋ 1 回に積む件数 ＋ 帳面の一時ファイルを FIFO にして Saving も止める（製品のコードは変えない）」。2 つめの選択肢の「製品のファイルに差し込み口が入る」は、唯一の道としては書かない
- 処置: fixed deep-questions.json Q1（context を直し、名前付きパイプの選択肢を足した）・premises.md 2 行目

## R3. 「製品のコードは変えない」が試験用の口を許すかという範囲の問いが無い。Q1・Q3 と R1 で別々に出てくる
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/deep-questions.json（Q1 の 2 つめ・Q3 の 3 つめ・Q7）
- 根拠:
  - CH09.md「範囲の外」:「製品のコード（見回り・取り込み器）の振る舞いは変えない。試験だけを直す」。振る舞いを変えない `#[cfg(test)]` の口がこれに入るかどうかは書かれていない
  - Q1 の 2 つめは「依頼の読み方が変わる」、Q3 の 3 つめは「依頼の範囲を越える」と、それぞれ別の読み方をしている
  - R1 の (a) も、この読み方しだいで採れるかどうかが決まる
  - Q7 は「不具合が見つかったら」の仮定を問うだけで、change の第 1 回が問うべき「変えてよい振る舞い・足してよいものが本当に無いか」に直接は答えていない
- kind: technical
- 提案: 第 1 回の冒頭に範囲の問いを 1 つ足す（推奨付き）。例: 「製品のファイルに、振る舞いを変えない試験用の口（`#[cfg(test)]` の差し込み・観測の印）を足すのは範囲の内か」。Q1・Q3・R1 の選択肢は、この答えに従う（`follows`）形にする。答えが分かれて、ちぐはぐにならないようにする
- 処置: fixed deep-questions.json Q0（範囲の問いを冒頭に足し、Q1・Q3・Q8 の口を足す選択肢は Q0 に従うと書いた）

## R4. 族「実時間で性質を確かめている試験」の兄弟が欠けている（打ち切り・「起きない」の窓・暗黙の上限）
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/deep-questions.json（Q1・Q3・Q4 の `family.siblings`）
- 根拠:
  - **打ち切りが効くことを時間で見る型**: `crates/server/src/testdb_tests.rs:371-386` は、`statement_timeout` 1 秒で `pg_sleep(5)` を打ち切ったことを「経過 < 5 秒」で見ている。`crates/collector-windows/src/time_sync.rs:650-656` は、打ち切り 200ms で `sleep 30` を止めたことを「経過 < 5 秒」で見ている。上限が「待ってしまったら掛かる時間」の下に挟まっている。依頼の 3 つの形のどれにも当たらない（直すなら、相手の長さと上限を両方延ばす第 4 の形になる）
  - **「起きない」を短い窓で見る型・下限で見る型**: `collector-android/app/src/test/kotlin/dev/ashiato/collector/UnavailableLogGateTest.kt:70-73` は「200ms のうちに終わらない」を `assertFalse` で見ている。`crates/server/src/web_session_tests.rs:273` は「経過 ≥ 300ms」を見ている。どちらも混んでいて落ちることは無い。混むと見逃す側へ弱まる。一覧の分類が要る
  - **ライブラリの既定の暗黙の上限**: `web/vitest.config.ts:11-17` には `testTimeout` が無い。`web/src/__tests__/setup.ts` にも `configure` が無い。testing-library の `waitFor` / `findBy*` は既定で 1 秒、vitest の試験は既定で 5 秒になる。`grep -rhoc "waitFor\|findBy" web/src/__tests__` の合計は 317（23 ファイル）。試験のコードに数字が出てこないので、premises の grep の型（`elapsed() <`・`for _ in 0..N`）には掛からない
- kind: technical
- 提案: 3 問の `family.siblings` に 3 つの型を足す。打ち切りの型は、どう扱うか（相手と上限を両方延ばす／一覧に載せるだけ）を 1 問にするか、Q3 の選択肢に含める。「起きない」の窓・下限の型は、「混んで落ちることは無いので直さず一覧に分類して載せる」を既定として書く
- 処置: fixed deep-questions.json Q1・Q3・Q4 の family（兄弟を 3 つ足した）・Q3（打ち切りの型を含めた）・Q9（暗黙の上限を足した）。「起きない」の窓・下限は既定（直さず分類して一覧）として兄弟に書いた

## R5. premises 4 行目と Q2 の context に、画面・携帯の規模が無い。Windows だけで走る試験の直し方の確かめ方も無い
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/premises.md（4 行目）・deep-questions.json（Q2）
- 根拠:
  - premises の 4 行目は `crates/` にしか grep していない（「十数か所」）。Q2 の推奨は `web/src/__tests__`・`collector-android/app/src/test` も「探して直す」に入れている。ところが画面の単体試験には、暗黙の 1 秒の待ちが 317 か所ある（R4）。携帯にも `UnavailableLogGateTest.kt:34,36,58,62,69,75` の `await(5, SECONDS)` がある。範囲を決める規模がこの問いに載っていない
  - `crates/collector-windows/tests/runtime_windows.rs:19` は `#![cfg(windows)]` で、`cargo test --workspace`（Linux）では組み立てられない。docs/testing.md:171 によれば、走らせるのは Windows の側の clone の手だけ。`.github/workflows/` で Windows で走る job は配布物を作るもの（release.yml:45・nightly.yml:37）だけで、この試験は走らない
  - Q2 の推奨「`crates/` の下は Windows だけで走る試験も含む」を採ると、直した所を関門のどの検証も走らせない
- kind: premise
- 提案: premises 4 行目に `web/src/__tests__`（`waitFor`/`findBy` 317・既定 1 秒）と `collector-android/app/src/test` の数を足す。Q2 の context には「`runtime_windows.rs` を直すなら確かめは Windows の clone の手になる（関門では走らない）」と書く。選択肢は「`runtime_windows.rs` は直して Windows で 1 回走らせる／探して一覧に載せるだけ」で分ける
- 処置: fixed premises.md 4 行目・deep-questions.json Q2（context に規模と Windows の試験が関門で走らないことを書き、推奨を「Windows だけの試験は一覧に載せるだけ」に変えた）

## R6. Q5「経過時間の期限で一律 30 秒」に、中立でない箇所と日常の代価が書かれていない
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/deep-questions.json（Q5）
- 根拠:
  - `crates/collector-windows/src/runtime.rs:2499-2507`（`tick_until_read`）・`:3462-3476`（`tick_until_read_again`）は、ループの数を試験の中の時刻（`t(sec)`、`sec in from..from + 2000`）にも使っている。回数を実時間の期限に変えると、試験の中で進む時刻が今の 2,000 秒から、期限 30 秒 ÷ 5ms で最大 6,000 秒前後まで延びる。1 時間（3,600 秒）で効く規則（ST08 の上限 1 時間・M11）をまたぎうるので、試験の中の振る舞いが変わる。`tick_until_history_idle`（:2512-2520）は時刻を固定しているので、この問題は無い
  - 画面の試験で `waitFor` の上限を 30 秒にするなら、vitest の試験の既定 5 秒（`web/vitest.config.ts` に `testTimeout` が無い）も一緒に上げないと、試験の打ち切りが先に来て効かない
  - 日常: 上限を 30 秒にすると、本当に落ちている試験は 1 か所ごとに 30 秒待ってから落ちる。画面の試験は待ちが 317 か所あるので、下流の fix の往復で落ちた試験を回し直すたびに長くなる
- kind: daily
- 提案: Q5 の why に 3 点を足す。(1) 試験の中の時刻を兼ねるループは回数のまま残す（または時刻の上限を別に持つ）。(2) vitest の `testTimeout` を一緒に上げる。(3) 落ちるまでが延びる代価。選択肢に「試験の中の時刻を兼ねるループは除く」を入れる
- 処置: fixed D4 D9 仮（問いの段で deep-questions.json Q5 の why に 3 点・推奨に「時刻を兼ねるループは回数のまま」、Q9 に試験の既定も一緒に上げる、を足した。spec の段で design D4（仮）・D9（仮）に反転条件つきで写した）

## R7. 完了の判定のうち、「負荷で落ちることの再現」と「わざと壊して落ちる」の確かめ方が問われていない（族「検証コマンドが成立しない」）
- 成果物: openspec/changes/ch09-load-tolerant-timing-tests/deep-questions.json（Q6）
- 根拠:
  - `python3 scripts/upstream_misses.py report`: 族「検証コマンドが成立しない（11 件）」、観点「凍結前に tasks の全検証コマンドを verify-run で空走させ」「合成負荷でなく…パラメータを変えると結論が反転しないかを確かめる」
  - CH09.md の完了の判定は 3 つある。「負荷の下で続けて 10 回通る」「見回りが読みを待つようにわざと壊すと落ちる」「`cargo test --workspace` が通る」。Q6 は 1 つめの負荷のかけ方しか問うていない
  - 再現: 直す前の試験が、その負荷で落ちることを先に見ていないと、「10 回通る」は直したことの証にならない。2 の試験の落ち方は CPU の混み具合ではなく、DB の往復の長さで決まる（R1 の模型。回数の後の往復が `settle(25)` を超えたときだけ止まる）。全コアの busy loop で DB の往復が延びるかどうかは確かめられていない
  - 壊す: わざと壊すには製品のコードを一時的に書き換えて戻す必要があり、`verify-run` の 1 コマンドでは表せない。壊した状態で見回りが gate の `recv`（`runtime.rs:2378-2381`）で止まると、見張り（Q1 の 30 秒）が無い限り検証が返らない
- kind: technical
- 提案: Q6 の選択肢・why に 2 点を足す。(1) 台本はまず直す前の試験に当て、落ちる回が出ることを証跡に残す（出なければ負荷の作り方を変える。DB の往復を延ばす負荷も候補にする）。(2) 「壊すと落ちる」の確かめ方。例: 一時の作業木に差分を当てて rc≠0 を期待する台本、戻したことを `git diff --exit-code` で見る。どちらも凍結前に一度空走させる
- 処置: fixed deep-questions.json Q6（直す前に再現を先に証跡に残す）・Q10（壊すと落ちるの確かめ方を足した）

---

不要・分類違い・不可逆の根拠が無い・文字で決めさせている画面の問い・既定の無い仮の問い・不可逆の記述が無い: 該当なし（確かめた範囲: Q1〜Q7 の `kind`・`recommended`・選択肢。7 問とも `open` で `loss` が無く、どれにも推奨がある。画面の構造を問う問いは無い。Q7 の推奨は依頼の「範囲の外」と同じだが、問い方の不足として R3 で扱い、不要とはしない）。
