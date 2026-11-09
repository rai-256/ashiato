# impact-review — st56-collector-connection-setup

seen: IMP-01 medium, IMP-02 medium, IMP-03 low, IMP-04 low

確かめて支えられたもの: IMP-01 の「5180 の `/api` は vite preview の中継が API へ渡す」は `web/vite.config.ts:9-15,38`（preview にも同じ proxy）と `docs/network.md:16` で言える。`unknowns` に書いた通り、通すか否かは未確認で `unconfirmed` が正しい。IMP-02 の `main.rs:14`（`from_env()?` で変数が欠けると終わる）、IMP-03 の `HealthManifestTest.kt:77`（`<exclude` を 2 つと見る）も実物どおり。

## R1 — 前提どおり変わる記録の損失（原本の欠け）が `lost`・IMP に無く、overview が「重大な判断事項は無い」と言い切っている
- 対象: impact.lost / impact.findings / impact.overview（S2・D10・R2）
- 種類: 言い切り
- 指摘: S2 は「前のサーバが受けた記録の写真の原本は、新しいサーバに記録が無く 7 日後に欠けになる」と、記録の取り戻せない損失を `derived` で書いている（D10 は `holds: no`、`leads_to: S2`）。根拠は `OriginalUploader.kt:21`（`RECORD_MISSING_GIVE_UP` 7 日）と design D13 の 133 行。ところが `lost`（4 件）にこれが無く、IMP も立っていない。しかもこの場面（開発のサーバからインストールした形のサーバへ切り替える）は、本人が配る形へ移る最初の動きそのもの。overview の「重大な判断事項は無い」は、R2・R4・R5 が `blocks_legit: true`、S2 が取り戻せない損失を持つ状態で、限定（「Q7 で選んだ帰結として」など）を付けずに読める。
- 直し方の案: S2 の原本の欠けを `lost` に足し、IMP（medium。選択肢は「手順書に、切り替える前に原本を送り切るよう書く」「受け入れる」）を立てて、overview の言い切りを「判断事項は IMP-xx と S1・S2 に出した」の形に直す。
- 処置: fixed: IMP-05（medium。原本の欠け）を立て、lost に足し、S2 と D10 から繋いだ。overview の言い切りを「判断事項は IMP-01〜06 で、どれも high ではない」に直した

## R2 — D8（録画の台本）の `holds: yes` が、この change が崩す前提と食い違う
- 対象: impact.dependencies[D8]
- 種類: 根拠
- 指摘: D8 の `assumes` は「環境変数だけで送り先が決まる」で `holds: yes`。しかし D17（design.md:163-166）は、印が無い起動は環境変数を読まないとする。録画の起動は `tools/recording/st07/st07-desktop.ps1:38` の `Start-Process $exe`（引数なし）で、`record-env.sh:256-260` の `WINDOWS_ENV_*` だけで送り先を決めている。つまりこの change が何もしなければ前提は成り立たない（D7 と同じ型で `no`）。Task 12.1 が `WINDOWS_ENV_ASHIATO_CONFIG_FROM_ENV=1` を足して初めて保たれる。`holds: yes` と書くと、依存の一覧からは「壊れない」に見え、12.1 が落ちたときに ST07・ST08・ST50 の録画が黙って送らなくなることが読み取れない。なお環境変数の渡し方自体は `.harness2/scripts/record-run:337-338` が `WINDOWS_ENV_*` を全部渡す作りなので、12.1 で足りる見込みは言える。
- 直し方の案: D8 を `holds: no`（直しは Task 12.1）にして `leads_to` を付けるか、`assumes` を「印を付けた環境変数」に書き直す。
- 処置: fixed: D8 の assumes を「印を付けた環境変数で送り先が決まる（Task 12.1 が渡す）。印が無ければ読まない」に書き直した（holds: yes は 12.1 が前提）

## R3 — 依存の見落とし: `docs/network.md` §3・§4 の手順が、この change で古くなる
- 対象: impact.targets[T1] / impact.dependencies（新しい行が要る）
- 種類: 依存の見落とし
- 指摘: `docs/network.md` §3 は「`ashiato.baseUrl` を直して APK を入れ直す」、§4 手順 2 は「`API_TOKEN` を変えて、APK と C-02 を入れ直す」と、電話を落としたときの締め出しの手順を書く。ST56 の後は、配る APK では貼り直しで足りる（`-P` の値は保存した設定が無いときだけ使われる）。入れ直しても保存した設定が先に読まれるため、手順どおり APK を入れ直しただけでは新しい `API_TOKEN` が効かず、携帯は古い資格情報のまま 401 で断られ続けうる。tasks に `docs/network.md` の直しは無い（`docs/collector-setup.md` と `docs/release.md` だけ。tasks.md:124-126）。
- 直し方の案: `network.md` §3・§4 手順 2 を T1 の読む側として `dependencies` に足し、Task 12 に「`network.md` に、貼り直しが要ることを書く」を足す。
- 処置: fixed: 依存 D12（docs/network.md §3・§4）と IMP-06（medium）を足した。tasks は reading の段で直さない（spec に触らない）ので、IMP の選択肢「下流で network.md に貼り直しを足す」で人間に返す

## R4 — IMP-02: 重大度は medium でよいが、気づける経路の根拠が足りない
- 対象: impact.findings[IMP-02]（`options` の「稼働状況の格子の途絶で気づく」）
- 種類: 根拠
- 指摘: 前の版は `main.rs:14` の `from_env()?` で落ち、`init_logging`（`main.rs:19` 以降）の前なので `collector.log` にも何も残らない。台本の窓は最小化で動く（`main.rs:21-23` の注記）。気づける経路は格子の途絶だけで、これを「受け入れる」の理由に書くなら、PC の途絶が格子に出ることを実物で確かめた根拠（refs か evidence）が要る。今は挙げていない。重大度の置き方（medium）自体は、台本に変数を書き戻せば直る点で妥当。
- 直し方の案: `unknowns` に「落ちた理由がログに残らない」を足し、格子で途絶が見える根拠を evidence に足す（見えなければ手順書の案を既定にする）。
- 処置: fixed: IMP-02 の unknowns に「ログに残らない」「格子で見えるかは確かめていない」を足し、受け入れの選択肢から格子の根拠を外した

## R5 — R3（携帯の平文・自分自身の送り先の断り）の「もともと送れない」が言い過ぎ
- 対象: impact.rules[R3].legit
- 種類: 言い切り
- 指摘: 「携帯はもともと平文の送り先へ送れない」と書くが、`docs/network.md:26`（「`http://` の loopback 以外は組み立てが落ちる」）は loopback の `http://` は組み立てで許すと読め、録画の Android も `ANDROID_REVERSE=18787:…`（`record-env.sh:249`）で携帯の 127.0.0.1 を使う。貼る道では開発のエミュレータや adb reverse の携帯が断られる（`-P` は影響を受けない）。「新しく止まる正当な利用は見つけていない」ではなく「貼る道では断られる。`-P` で代替できる」と書くのが事実に近い。
- 直し方の案: `legit` を「開発のエミュレータで 127.0.0.1 を貼る道は断られる（`-P` で代替）」と書き直す。
- 処置: fixed: R3 の legit を「エミュレータ・adb reverse で 127.0.0.1 を貼る道は断られる（-P で代替）」に書き直した
