# ST52 仮で進めた決定の確かめ（deep-review / provisional）

対象: `provisional.jsonl` の D1・D2・D3・D4・D5・D6・D21・D22・D23。問いを作った AI の「失わない」を信じず、コード・design・tasks を grep と実行で辿った。

## D1
- 判定: 否
- 根拠: 条件 1（失わないか）と条件 4（土台の決定）の両方で否。(a) 条件 1: 祖先の印だけを数える `git tag --merged HEAD` は、CI の浅い checkout では空を返し、検査は「配った印が 0 個」として黙って通る。この検査が走る `rust` の job は `.github/workflows/ci.yml:68-69`（`actions/checkout` に `fetch-depth` が無い = 深さ 1）で、印を `ci.yml:93`（`git fetch --force --no-tags --depth=1 origin '+refs/tags/v*:refs/tags/v*'`）で取り寄せる。手元で同じ形を再現した: 印 v0.1.0 を c1 に打ち、c3 を `git clone --depth=1` して同じ fetch をすると、`git tag --list 'v[0-9]*'` は `v0.1.0`、`git tag --merged HEAD --list 'v[0-9]*'` は空になる。この状態だと、配った版の移行の書き換えと送信契約の版の上げ忘れを、`tools/check-released-migrations.sh:18-22` と `tools/check-contract-version.sh:108-110` が見逃す。配った版を入れた他人の DB とずれる（discarded）。tasks 4.1・4.2 の検証は手元の完全な木で rc=0 を見るだけで、`ci.yml` の取り寄せの深さを直す項目は無い（tasks.md に `depth` は 0 件）。直す方法は安い（その job の checkout を `fetch-depth: 0` にする、または印の祖先まで取り寄せる。印は今 0 個なので `--merged` の集合が空かどうかで区別できない —— 浅い木で落とす検査を足す）が、台帳と tasks にそれが無いまま「数える印を絞るだけ」と書いている。(b) 条件 4: keeps の「祖先でない印の移行・封筒は、その枝の外の版なので凍結の対象に入れなくても今の木で失われない」が成り立つのは、修正の枝が移行と封筒を変えないとき（D6（仮））だけ。D6 が無ければ、修正の枝の版で当てた移行は main の凍結に入らない。その版を入れた DB は、次の main の版で `Ahead` になる（`crates/server/src/migration_ledger.rs:365-367`）。それなのに `rests_on` は `[]` で D6 が挙がっていない

## D2
- 判定: 確か
- 根拠: 失わない側（`v*` の削除・更新を拒む）を選んでも、印は残るので凍結も契約の版も外れない。拒んでも動かせなくなるのは `v*` だけで、毎日動かす `nightly` は型の外（design D15、`nightly` は `v*` に当たらない）。下書き（D16）は印を作らない前提で、P16 が外れたときは「消す前に人間に返す」と design の Risks に書いてある。費用はルールセット 1 つと検査 1 つで、範囲も要件も越えない。ただし design D2 の確かめの口 `gh api repos/{owner}/{repo}/rules/tags/<印>` は道が実在しない。`gh api repos/rai-256/ashiato/rules/tags/v0.1.0` は 404（Not Found）で、`rules/branches/main` は `[]` を返す。これで失われるものは無い（検査は公開を止める向きに外れる）。それでも tasks 9.3 で `GET /repos/{o}/{r}/rulesets` と `/rulesets/{id}`（対象 `tag` と条件 `refs/tags/v*` を読む）に替える必要がある。design の反転条件は「権限の不足」しか想定していない

## D3
- 判定: 確か
- 根拠: ST52 では鍵を作らない（design Non-Goals「鍵そのものを作ること…（本人の手）」・D23「開発版の鍵を作るのは ST54」）。なので値を何にしても、ST52 の間は何も出ない（exported は起きない）。`-keysize 4096 -validity 10000`（27.4 年）は「RSA・25 年以上」を満たす。D3 の反転条件は「作る前に引数を変える」で、作った後に頼っていない。費用は引数だけ。一次情報（Play App Signing に持ち込める条件）はまだ確かさ 0。premises に足すのは tasks の責務で、鍵を作る前に済むので失うものは無い

## D4
- 判定: 確か
- 根拠: DB が配布版より先に進んでいると、`apply_locked` は記録を読んだ直後に `Verdict::Ahead` で `MigrateError::Ahead` を返す。`BOOTSTRAP`・付与・移行の前で、DB を書き換えない（`crates/server/src/migration_ledger.rs:357-371`）。試験 `migration_ledger_tests.rs:530-549` は、ahead のとき付与が流れないことを確かめている。起動の検査も同じ判定で止まる（`:292-299`）。移れない間も本人の DB は main の版のまま残り、失われない。費用は手順書の数行（tasks 11 の `docs/release.md`「本人の環境を移す」）

## D5
- 判定: 確か
- 根拠: ST52 は公開しない。本物の印は ST53 の後（Q5 の答え）で、確かめは下書きだけ（design D16）。PC の収集アプリを入れる操作も持たない。なので ST52 の間は、PC の新しい版から古い版への入れ直しが起きる配布物が出ない。旧版が知らない欄を落とす経路（`crates/collector-windows/src/outbox.rs` に `deny_unknown_fields` が無い。`grep -n deny_unknown_fields crates/collector-windows/src/*.rs` の当たりは `exclusion.rs` だけ）は、ST58 への申し送りで受ける。tasks.md:174 は `docs/handoff/ST58.md` に D5 の節を足し、`grep -q "D5" docs/handoff/ST58.md` で検証する（今の ST58.md には未だ無い）

## D6
- 判定: 確か
- 根拠: 失わない側（修正の枝では移行と封筒の形を変えない）を選んだ場合に失うのは、「枝の上で形を変える」という選択肢だけ。main では今どおり変えられる。修正の枝はまだ一度も切っていない（配った `v*` の印も 0 個。premises P11）ので、凍結した行も外へ出た版も無い。`docs/contract/envelope.json` は実在し、比べの費用は `plan` の 2 項目。FR-105 は「一時的に切る」で、移行を当てることを求めていないので、要件の本文も変わらない。補足（判定は変えない）: design D6 の元の印を探す `git describe --tags --abbrev=0 --match 'v<MAJOR>.<MINOR>.*'` はベータの印にも当たる。手元の木で `v0.2.1-beta.1` が返るのを見た。「正式版の印に限る」を守るには `--exclude '*-*'` が要る

## D21
- 判定: 確か
- 根拠: 正式な鍵を使う job は `release.yml` の `publish`（`on: push: tags: ['v*']`、design D14・D18）だけで、印の ref で走る。`main` の上で正式な鍵を要る job は無い。draft.sh（D16）は PR の配布物で、debug 鍵を使う。Q2 の答え「印と main にだけ開いた」とは同じ向きの狭い側で、requirements.md:1776 の補足（C2）も「印と main は開いてよい上限で、どこまで狭めるかは D21」としている。費用は Environment の 1 行。印の名前だけで開く経路（main を通らない commit に `v*` を打つ）は、印と main のどちらに開いても残る。D21 が新たに開くものではなく、`docs/handoff/ST53.md` の IMP-01 で本人が ST53 に回した

## D22
- 判定: 否
- 根拠: 条件 4。問いの本文は「Q5・Q2 の答え（と、本人の携帯にまだ収集アプリが入っていない Q1 の答え）のもとで」と言い、chosen も 3 つの答えに支えられている。「正式な鍵で署名」の読み替えは Q2（CI が Environment の鍵で署名する）に、「自分の環境をその版で動かす」の読み替えは Q1（携帯に入っていない）に依る（design D22 本文）。それなのに `rests_on` は `["Q5"]` だけで、Q1・Q2 が無い。design D22 の反転条件も Q5 しか挙げていない。このままだと、Q1・Q2 の答えが変わっても台帳から聞き直しが起きない。失うもの（条件 1）は無い（完了の判定の文面の読み替えで、配布元にも記録にも出ない）。条件 3: Q5 の推奨の detail「本物の公開は ST53 の確認で見る」とは同じ向き

## D23
- 判定: 確か
- 根拠: Q4 の答え「ST54 まで、開発版に携帯の APK を入れない」と、Q11 の答え「開発版は別の鍵」と同じ向き。requirements.md:969 の補足（開発版のリリースには ST54 まで C-01 を並べない）を、`check-artifacts.sh --channel nightly` が「APK があれば落とす」で機械が守る。ST52 は `nightly.yml` で APK を組まず、開発版の鍵も作らず（design D14・D18・D23）、使い捨ての鍵の `--self-test` で閉じる。なので識別子 `dev.ashiato.collector.dev` も開発版の鍵も外に出ない（Q11 の exported はまだ起きない）。gradle の `.dev` は `-dev.` を持つ版番号のときだけで、PR の `package` は `ASHIATO_BUILD_VERSION` を渡さない（D14）。今の識別子 `dev.ashiato.collector` は変わらない。費用は検査の期待の集合 1 つで、道具は推奨の版のまま
