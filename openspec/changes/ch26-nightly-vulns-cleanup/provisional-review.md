# CH26 仮の決定の確かめ（deep-review・provisional）

確かめた日: 2026-11-22。CI の定義は `git show origin/main:.github/workflows/ci.yml`（CH26 は main の上で実装する。design の Context）で読んだ。
D5 は台帳で resolved（本人が R2 = C に決めた）なので見ていない。

## D2
- 判定: 確か
- 根拠: (1) 失わない側を選んでも、`uncaptured`・`discarded`・`exported`・`rewrite-all` のどれも起きない。例外は `osv-scanner.toml` の 1 行で、ID・90 日以内の期限・`届かない: ` の理由を持つ。期限が切れると検査が `期限切れ` で rc=1 にする（`tools/check-vulns.sh:188-190` の自己試験、`openspec/specs/dependency-vulnerability-scan/spec.md:73` `:88`）。夜間の検査はそれを題 `脆弱性: 例外の表` の issue にする（同 spec `:219`、`check-vulns.sh:486`）。上げる道（`crates/collector-windows/Cargo.toml:26` の `rusqlite = "0.32"` を 0.41 へ）を閉ざすものも無い。(2) cost は例外の 1 行だけ。上げる側（範囲を 0.41 へ替え、`history/read.rs` の追従を見る）は依頼の「互換の範囲で上げる」を越える（P4）。そのため失わない側は「範囲を越える」側ではない。正典が `届かない: ` を「製品に届かない理由」と定める（`openspec/specs/dependency-vulnerability-scan/spec.md:71`）こととの食い違いは spec-review R6 で拾われた。D2 に理由が書かれ、GO 資料にも `届かない:` の定義と「型を変えることは範囲外」が出ている（`go.json` の terms・範囲の外）。本人は IMP-01 を含めて合意した（`agreement.json` の `minor_accepted`）。要件の本文は変えていない（tasks.md:108 の許すパスに `docs/requirements.md` も正典の spec も無い）。(3) `follows` は無い。(4) 土台は前提の P5〜P7・P16 で、決定ではない。D1（rustix）とは別の行で、互いに依らない
- 補足: 理由は P7（確かさ 1）に乗っている。これは前提の弱さで、仮の判定の 4 条件には当たらない。IMP-01 として本人が受けた。go.json の `open` が言うとおり、反転条件（Firefox の `IsRelative=0`）を誰がいつ見るかは決まっていない

## D3
- 判定: 確か
- 根拠: (1) 何も捨てない。chrono は直る版が無く（P8）、例外の行は 90 日で `期限切れ` になって rc=1、夜間に `脆弱性: 例外の表` の issue になる（`tools/check-vulns.sh:190` `:486`、`openspec/specs/dependency-vulnerability-scan/spec.md:219`）。design D3 の「issue が立つ」はこの道と合う。(2) 置き換え（jiff / time）は 5 つの crate の本体と `sqlx`・`utoipa` の feature に及ぶ（`Cargo.toml:17`、`crates/server/Cargo.toml:18` `:27` `:30`、`crates/{installer,updater,update-check,collector-windows}/Cargo.toml` の `chrono.workspace = true`。P9）。そのため失わない側（例外の 1 行）は高くつかず、範囲も越えない。正典の型との食い違いは D2 と同じく R6 で拾われ、本人に見せてある。(3) `follows` は無い。(4) 土台になる決定は無い

## D7
- 判定: 確か
- 根拠: (1) CI の定義の中の決め方なので、記録も配る物も失わない。`package-windows` の使い回しは鍵が完全に一致したときだけで、`restore-keys` を持たない（origin/main の `ci.yml:533-546` の「CH17 D2: …restore-keys は書かない」）。そのため決め方を変えても古い組み立ては引かれず、次の run から効く。(2) cost は `changes` の出力 2 つと `package-windows` の数 step。`changes` は既に `fetch-depth: 0` で履歴を全部持ち（`ci.yml:32`）、分岐点の計算に新しい取り寄せは要らない。head は `<head.sha>` で渡す（tasks.md:82）。`ci.yml:35` の `github.sha`（PR の run では merge の commit）ではないので、分岐点は spec の文言（`specs/distribution/spec.md:52`「PR の base との分岐点から PR の head まで」）と合う。大きい版の違う lock で CI を落とすことは、大きい版を上げる Story の側の費用になる。本人は IMP-06 として受けた（`agreement.json`）。(3) `follows` は無い。(4) 土台は D5（本人の決定）だけで、`rests_on: ["D5"]` と合う

## D8
- 判定: 否
- 根拠: 条件 4。`rests_on` は `["D5"]` だけだが、D8 は D7（仮）の上に立つ。台本の `changed <base> <head>` は D7 の「分岐点から head の差分で決める」をそのまま実装し（`design.md:106`、`:92` と同じ規則）、`decide` が rc≠0 を返すのは D7 の「同じ中身・大きい版が違えば落とす」のためにある。`check-workflow` は D7 の `pgsql == 'true'` の条件と「PR の run でだけ」を ci.yml の形として固める（`design.md:110`）。D7 が確認バッチで覆れば、D8 の台本のサブコマンドと形の検査は作り直しになる。それでも台帳からは D8 を見直す筋が辿れない。(1)・(2) は通る。台本は配る物に入らず（`tools/release/` の CI の道具）、失うものは無い。cost は台本 1 本と `chain` の 2 行。`docs/release.md` の小節の書き換えは本人の補足（「docs/release.md の手順を CI で機械が行う」。`deep-answers-2.txt`）の範囲にある。(3) `follows` は無い
- 直し方: `rests_on` に `D7` を足す（`provisional.py` で足し直す）

## D9
- 判定: 否
- 根拠: 条件 4。`rests_on` は `["D5"]` だけだが、鍵に足す値 `prev-pgsql=<前の lock の sha256、取らなければ none>` は、D7（仮）が決める「PR の run で、差分が lock を変えたときだけ分岐点の lock を取る」の出力から作る（`design.md:124`「前の lock を取る step の出力から」）。「取らなければ none」の意味も、鍵が木の外の入力を持つこと自体も、D7 の「前の lock は分岐点の lock」から来る。D7 が覆れば（たとえば前の lock を base の先端から取る・木の中に置く）、D9 は要らなくなるか、入れる値が変わる。(1)・(2) は通る。鍵の windows-package は `restore-keys` を持たず完全一致でだけ引く（origin/main の `ci.yml:533-546`）。そのため鍵の値を変えても、起きるのは組み直し（約 25 分）だけで、失うものは無い。P21（確かさ 4）は、最終形の写しで `windows-build-key.sh --check-workflow` が rc=0 だったことを記録している。正典の鍵の Requirement を `## MODIFIED` で書き換えるのは openspec の正典の変更で、`docs/requirements.md` の要件の本文ではない。GO 資料の「5 つの Requirement（新しい 4 と書き換えた鍵の 1）」で本人が合意している（`go.json` の coverage）。(3) `follows` は無い
- 直し方: `rests_on` に `D7` を足す

## D10
- 判定: 否
- 根拠: 条件 4。`rests_on` は `["D5"]` だけだが、chosen の「zip の同梱の版…が通ることだけを見る」の「zip の同梱の版」は D8（仮）の `zip-version`・`check-zips`（保存の前と save の後の 2 か所。`design.md:110-117`）のことで、D10 の費用の見積もり（「`pgsql` の置き場だけを入れ替え損ねても緑になりうる」）はこの確かめがある前提で言っている（`design.md:132`「`package-windows` は 2 つの zip の同梱の版を確かめる」）。D8 が覆って版の確かめが無くなれば、「古い側の zip がそもそも前の版でない」も CI を緑で通りうる。そうなると D10 の費用は書かれた範囲より広がる。(1)・(2) は通る。何も捨てない。見る側（`update-apply-install.sh` に `postgres.exe -V` の読みを足す）は、この change が変えないとした組み立ての台本に及び、本人の補足「既存の update-apply-install.sh で通す」を越える。そのため失わない側は「見ない」の側。本人は IMP-05 として受けた（`agreement.json`）。(3) `follows` は無い
- 直し方: `rests_on` に `D8` を足す（D8 が D7 に依るので、D7 も間接に土台になる）
