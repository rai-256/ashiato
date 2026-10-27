# CH07 — design

> questions の段（2026-10-25）は D1（review/deep.md R3）だけを置いた。spec-review（`review/spec.md` R1〜R10）を受けて D1・D3〜D5 を直した。spec の段（2026-10-25）が CH06 の gate 済みの spec（`c666673`。`deps.json`）と、
> CH06 の木の `tools/verify-env.sh`・`tools/check-verify-env-reset.sh`（同じ SHA）を読んで残りを書いた。前提は `premises.md`（P1〜P9）、聞かずに決めたものは同じファイルの C1〜C9。

## Context

- 何のためかは proposal.md の Why。振る舞いは `specs/verify-environment/spec.md`（Requirement 2 つ・Scenario 12 本）
- CH06 の `tools/verify-env.sh`（`c666673`）: 判定 `decision=$(plan)` → `reset` なら `reset_db`（DB のコンテナが無ければ `切った接続: 0（DB のコンテナが無い）` を出して戻る。
  あれば volume を控え、別のコンテナが使っていれば何も消さずに `exit 1`、使っていなければ `cut_line` → `rm -s -f db` → 控えた volume を名前で消す）→
  `clear_blocking_network` → `up -d --wait db >/dev/null` → 待ちのループの中で `tools/db-roles.sh >/dev/null` → **`exit 0`（成功の出口はここ 1 つ）**。
  切った接続の 1 行と `keep:` の行は標準エラー。標準出力に出す行はいま 1 つも無い（`--plan` を除く）
- CH06 の `tools/check-verify-env-reset.sh`（`c666673`）: 本体は段 1.〜8.（CH06 design D8）。各段の呼び出し `call` が標準出力を `$tmp/out`、標準エラーを `$tmp/err` に分けて残す。
  `--self-test` は一時の repo と worktree（`demo`・`demo-st05`・`demo-verify`・`demo-ch01`・`demo-ch02`）で `--plan` と `env-init.sh` を確かめる
- `.harness2`（#112 を含む。premises.md P1・P8）の `verify-run` は、準備の出力（標準出力と標準エラー）から行頭の `HARNESS_RESET:` を数え、無ければ「注意」を出す（P2・P3）。
  `init.py --check` の `check_verify_env` は `tools/verify-env.sh` の字面だけを見る（P4）。いまの木では `[人間] tools/verify-env.sh が作り直したものを宣言していない…` が出る

## Goals / Non-Goals

Goals:
- 作り直したときだけ、準備の標準出力に `HARNESS_RESET: testdb` が 1 行出る。作り直さない・作り直せないときは 1 行も出ない（依頼の完了の判定 1）
- その出る・出ないを、CH06 の検査の本体と `--self-test` が段ごとに機械で確かめる（判定 2）
- `.harness2/scripts/init.py --check` の `tools/verify-env.sh` の項目が `[ok]` になる（判定 3。C7）

Non-Goals:
- CH06 の振る舞い（判定・`--plan`・手順・切った接続の 1 行・作り直せないときの rc・`keep:` の行）を変えること（C4）。CH06 の 24 本の Scenario は CH06 の検査がそのまま走らせる
- `tools/stack.sh` など検証コマンドの側に宣言を出すこと（C4）、`HARNESS_RESET: none` を出すこと（C3）
- harness2（`verify-run`・`init.py`）を変えること

## Decisions

### D1（仮）. harness2 #112 が CH07 より先に入っても、順を変えず CH06 へ差し戻さない（premises.md C8）

#112 は spec の段の時点で merge 済みで `.harness2` に入っている（P1）。宣言の無い木の全部で `verify-run` のたびに「注意」が出ているが、注意は表示と記録だけで status を変えない（P3）。
CH06 の下流がその注意を見て宣言の行を CH06 に入れても失うものは無く、CH07 はその行を確かめる側になる。CH07 は注意を止めるために #112 や CH06 の順を変えない。
spec の段で確かめた: CH06 が gate を通った `c666673` の `tools/verify-env.sh` に `HARNESS_RESET` の字は無い。#112 が先に入る側は起きたが、CH06 が後で宣言を足すかは CH06 の下流の間は決まらないので（仮）のまま置く（review/spec.md R1）。
反転条件: CH06 が後で宣言の行を足した場合は、tasks 1.1 がその行を `reset_db` の印に結び直す（宣言の字は本文に 1 つだけ。tasks 1.1 の検証が数える）。
注意が CH06 以外の走っている change の下流を止める・誤った直しを起こす（注意を見て `verify-env.sh` に宣言を足す・`none` を出す）と分かったら、harness2 側に返す。

### D2. 宣言は `reset_db` が立てた印を見て、成功の出口（待ちのループの `exit 0`）の直前で出す（C1・C2・C9）

`reset_db` が「DB のコンテナが無かった」か「控えた volume を 1 つ以上消した」ときにだけ、変数（例: `reset_done=1`）を立てる。
待ちのループの中で `tools/db-roles.sh` が通った後、`exit 0` の直前に、その変数が立っていれば `echo "HARNESS_RESET: testdb"` を 1 回出す（字下げした echo でよい。出る行は行頭から始まる）。
- 控えた volume が 0 個のときは、`rm -s -f db` まで CH06 のとおり進むが、変数を立てない（spec の「volume を持たない DB のコンテナでは…」。中身が残りうるまま宣言しない —— R8 (g)。厳しい側）。
  tmpfs のように実際には中身が消える構成でも宣言しない（判定は消した volume の数で決め、中身が消えたかを推し量らない。宣言が無ければ `verify-run` は注意を出すだけで status を変えない —— P3）
- DB のコンテナが止まっていた場合は、`cut_line` が「止まっていた」を出した後に volume を消すので、変数が立つ（R8 (e)）
- `keep` の木（印なし・CI でない main の checkout・git の木でない）は `reset_db` を呼ばないので、変数は立たない（git の木でない場合は CH06 の判定で main の checkout と同じ `keep` の経路。R8 (f)）
- どこかで `exit 1` で落ちれば `exit 0` に届かないので、宣言は出ない（volume を消した後の `up` や `db-roles.sh` の失敗を含む）

採らなかったもの: `reset_db` の中で消した直後に出す（後の `up`・`db-roles.sh` が落ちても宣言が出力に残る。`verify-run` は rc≠0 なら読まないが、依頼の「非 0 で終わる場合は出さない」と食い違う）/
判定が `reset` なら出す（volume が 0 個でも出る。R8 (g)）/ 宣言を `db-roles.sh` などの下請けに出させる（その標準出力は `>/dev/null` で捨てている。`init.py` は `tools/verify-env.sh` の字だけを読む。P4・C2）/
標準エラーに出す（`verify-run` は数えるが、依頼が標準出力と書き、検査が出所を分けて確かめられなくなる）。

### D3（仮）. 検査の本体に足す確かめ —— 既存の段に宣言の数を足し、2 つの段を足す（C5）

宣言の数え方: 各段の `call` の後に、`$tmp/out` の中で `HARNESS_RESET: testdb` だけの行の数（`grep -cx`）と、`$tmp/out` と `$tmp/err` の中の `^HARNESS_RESET:` の行の数を見る。

| 段（CH06 design D8） | 宣言 | Scenario の印 |
|---|---|---|
| 1. 空にする | 標準出力に 1 行（前の状態に依らず、コンテナが無いか volume を持つ） | —— |
| 3. 印なし | 0 行 | 印が無ければ、宣言を出さない |
| 3. CI だけの印つき `keep` | 0 行 | CI でない main の checkout では、印つきでも宣言を出さない |
| 4. 別のコンテナが volume を使っている（rc≠0） | 0 行 | 作り直せずに非 0 で終わるときは、宣言を出さない |
| 5. 繋いでいるものがある | 標準出力に 1 行・標準エラーに 0 行 | 溜まった DB を作り直したら、宣言を 1 行出す |
| 6. もう一度 | 標準出力に 1 行 | 2 回続けて作り直しても、どちらも宣言を 1 行出す |
| 7. DB のコンテナが無い | 標準出力に 1 行 | DB のコンテナが無くても、立てたら宣言を 1 行出す |
| **9.（新）止まっていた DB** | 標準出力に 1 行・`切った接続: 0（DB は止まっていた）` | DB のコンテナが止まっていても、作り直したら宣言を 1 行出す |
| **10.（新）volume を持たない DB** | 0 行・`切った接続: ` の行はある | volume を持たない DB のコンテナでは、作り直しても宣言を出さない |
| **11.（新）途中で落ちる** | 0 行・rc≠0・控えた volume は無い | 作り直しの途中で落ちたら、宣言を出さない |
| 8. 塞いでいる network（worktree だけ） | 空の network を消して立てる呼び出しで 1 行、rc≠0 の 2 回で 0 行、最後の立て直しで 1 行 | ——（4. の印に含める） |

新しい段は 7. の後・8. の前に 9.・10.・11. の順で置く（8. は DB と network を手放すので、その前に DB を volume ありの形に戻しておく）:
- **9.**: `docker compose stop db`（volume は持ったまま）→ 印つきで呼ぶ
- **10.**: 一時の場所に、service `db` に `tmpfs: /var/lib/postgresql/data` だけを書いた compose のファイルを置き、DB のコンテナと volume を消してから、
  環境変数 `COMPOSE_FILE` に今の木の値（`.env` の `COMPOSE_FILE`、無ければ `docker-compose.yml`）とそのファイルを並べて `docker compose up -d --wait db` で立て直す（`Mounts` が空になる。P9）。
  同じ環境変数を渡して印つきで呼ぶ → rc=0・`切った接続: ` の行がある・宣言は 0 行・DB のコンテナの volume は 0 個のまま。
  戻すときは同じ環境変数で `docker compose rm -s -f db` → 環境変数を渡さずに印つきで呼ぶ（コンテナが無いので宣言 1 行。volume を持つ形に戻る）
- **11.**: volume の名前を控え、`ENV_FILE=$tmp/<無い名前>` を渡して印つきで呼ぶ（`ENV_FILE` を読むのは `tools/db-roles.sh` だけで、明示したファイルが無ければ非 0。volume を消して DB を立てた後の役割の段で落ちる）→ rc≠0・宣言 0 行・控えた volume が無い。
  戻す: `ENV_FILE` を渡さずに印つきで呼び、rc=0・宣言 1 行（D2 の「`exit 0` の直前で出す」をこの段が固定する。review/spec.md R2）
- sentinel と同じく、10. の compose のファイルは `$tmp` に置き `trap` で消える。10. の途中で落ちて tmpfs のコンテナが残っても、次の印つきの準備が（宣言を出さずに）作り直し、その次から volume を持つ形に戻る

採らなかったもの: 木に一時の `docker-compose.override.yml` を置く（`.gitignore` に入っておらず未追跡のファイルが見える・CH06 D6 の `env-init.sh` が `.env` の `COMPOSE_FILE` に並べ、消した後の最初の準備まで残る・手で置いた override のある木（`ashiato2-integ`）では上書きになる）/
volume が 0 個の場合を確かめない（R8 (g) の決定が test で固定されない）。
反転条件: `tools/db-roles.sh` は `.env` を `set -a` で読み直すので、その中の `docker compose` は環境変数でなく `.env` の `COMPOSE_FILE`（tmpfs のファイルを含まない）で動く。
`exec` はラベルで同じコンテナを見つけるので通る見込み（spec-review が一時の compose project で実測して rc=0。review/spec.md 観点 4・premises.md P9）だが、10. の印つきの呼び出しが `db-roles.sh` で落ちたら、override の形（手で置いた override がある木では SKIP）に替える。

### D4. `--self-test` に足す確かめ（C5）

- **`--plan` の出力**: 既にある 3 つの `--plan` の Scenario の呼び出し（`demo` と 3 つの worktree、印あり・なし、`CI=true` あり・なし）の出力が、どれも 1 行で `reset` か `keep:` で始まり、`HARNESS_RESET:` の行を含まないことを、
  新しい Scenario「判定だけを出すときは、宣言を出さない」の印の下でまとめて見る
- **git の木でない場所**: 一時の場所（`git init` しない）に `tools/verify-env.sh` を写し、`CI` を落とし `GIT_CEILING_DIRECTORIES` にその一時の場所の親を渡して `HARNESS_VERIFY=1 … --plan` が `keep:` で始まる 1 行で宣言を含まない（Scenario「git の木でなければ、印つきでも作り直さない判定になる」。`keep` の経路は `reset_db` を呼ばないので、判定までで宣言が出ないことが決まる —— D2）。`GIT_CEILING_DIRECTORIES` が要るのは、`verify-run` が渡す `TMPDIR` が worktree の中（`.superpowers/sdd/tmp/`。`.harness2/scripts/evidence.py:372`）にあり、渡さないと写した `verify-env.sh` の `git rev-parse` が上の worktree を見つけて `reset` を返すため
- **宣言の字**: `tools/verify-env.sh` の注記でない行に `echo "HARNESS_RESET: testdb"` がちょうど 1 つあり、その行が `>/dev/null`・`>&2`・`$(` を含まない（C2。`init.py` が字面で読み、出力を捨てる塊の中に置かない）
- **ハーネスの点検**: `.harness2/scripts/init.py` があり `check_verify_env` を持つなら、repo の根で `python3 .harness2/scripts/init.py --check` を走らせ、`[人間] tools/verify-env.sh` を含む行が無く、`[ok] tools/verify-env.sh` を含む行がちょうど 1 つ（rc は見ない。C7）。
  無ければ `SKIP` の 1 行を出して飛ばす（CI には `.harness2` が無い。P8）。worktree の `verify-run` の証跡がこの Scenario を通す（CH06 の本体の 3. と 8. と同じく、CI と `verify-run` の証跡を合わせて全部が通る）

### D5. 文書

`tools/verify-env.sh` の冒頭の注記に「作り直したら `HARNESS_RESET: testdb` を標準出力に 1 行（ハーネスの約束。ch07）」を 1 行足す。
`docs/flow-gates.md` の `tools/check-verify-env-reset.sh` の行の「見るもの」に「作り直したときだけの `HARNESS_RESET` の宣言」を足す。`docs/testing.md` の rust の行は検査の数が変わらないので触らない。
`tools/check-verify-env-reset.sh` の冒頭の注記に、ch07 の段（9.〜11.）と宣言の確かめがあることと、`--self-test` の「CI の rust job と手元で同じ結果」の例外（ハーネスの点検の段は `.harness2` のある木だけで走り、CI は SKIP）を足す（review/spec.md R10）。

### D6. capability と delta の形

proposal の Capabilities のとおり、CH06 が立てる `verify-environment` に Requirement を 2 つ `ADDED` で足す（CH06 の 6 つの Requirement の本文と Scenario は変えないので `MODIFIED` にしない。`## Purpose` は CH06 の delta が持つ）。
archive は CH06 の archive の後にグラフが回す。

## Risks / Trade-offs

- **volume を持たない構成では、中身が消えても宣言しない**（D2）→ `verify-run` は注意を出すだけで status を変えない（P3）。いまの `docker-compose.yml` と手で置いた override（`ports` だけ）はどれも volume を持つので、日常の木では起きない
- **CI には `.harness2` が無く、判定 3 の Scenario は CI では SKIP**（D4）→ worktree の `verify-run` の証跡（tasks 4.2）が通す。黙って通らないよう、`.harness2` が無ければ SKIP の行を出す
- **検査の本体が約 5 回分の作り直し（9.・10.・11. と 10.・11. の戻し）だけ長くなる**（CH06 の計測で 1 回 7〜12 秒）。CH06 D8 の反転条件（CI で 90 秒を超えたら 6. と 7. をまとめる）はそのまま当たる
- **CH06 が gate の後に `verify-env.sh` か検査の段を変える**（deps の spec の SHA が進む）→ merge gate が「依存の spec:」で落とし、fix が D2・D3 の段の対応を読み直す

## Migration Plan

DB の移行は無い。CH07 が入った後の最初の印つきの準備から、作り直した証跡に `reset: ["testdb"]` が残る。戻すときは宣言の `echo` の 1 行を消す（`init.py --check` が再び `[人間]` を出す）。
