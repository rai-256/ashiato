# impact-review — st59-change-password-from-web

seen: IMP-01 medium, IMP-02 medium, IMP-03 medium, IMP-04 high, IMP-05 low

## R1 — IMP-04 の帰結が小さく書かれている。上書きで戻るのは「取り消したはずの前の値」で、本人は気づけない
- 対象: impact.findings[IMP-04].text / severity / options、impact.dependencies[D2].affects
- 種類: 重大度
- 指摘: text は「新しい値を貼り直した収集アプリが断られる」だけを帰結にしている。`crates/installer/src/main.rs:1421-1431` を読むと、`password` は入力待ちの前に読んだ `settings` を丸ごと `replace_web_password_verifier(&settings, …)` で書き戻す。上書きされる `API_TOKEN` は、画面で変える前の値、つまり落とした携帯が持っている値である。
  携帯を落としたので画面で値を変え、その後に `password` を走らせた場合、取り消した資格情報が黙って有効に戻る。拾った人はまた送信できる。FR-128 の「前の値は変えた瞬間から断る」とも反する。
  携帯の側は 401 を本人に知らせない（`docs/network.md:42`）ので、本人は気づけない。変えた直後の画面の表示は正しいままで、気づく手がかりも無い。
  起きる条件は「入力待ちの間に画面で変える」で、稀という見立ては妥当。ただ、帰結が「貼り直しの手間」から「締め出しが戻る」に変わるので、unknowns と text にこの帰結を書くべき。
  options の 2 つ目（サービスを止めた後に読み直す）は `read_settings` を `stop_service_and_wait` の後へ移すだけで済む。受け入れるより直すほうが安い。acceptance の「受け入れるか本人が答える」に出すなら、この帰結を添えて出す。
- 直し方の案: text と unknowns に「取り消した前の API_TOKEN が戻る」を足し、options の 1 つ目の受け入れの理由を書き直すか、読み直しを Task に入れて reflected にする。
- 処置: fixed: IMP-04 を high に上げ、題と text に「取り消した前の値（落とした携帯の値）が黙って有効に戻る」を書いた。options の 1 つ目を「仕様を直す: 止めた後に読み直し 1 つの鍵だけ置き換える」にし、if_accepted（残るもの・範囲・ST58 の上流の合意までの見直し）を足した。仕様は reading の段では直さず、本人の答えに回す

## R2 — `situations[update]` と `[rollback]` の `can` は、そこへ至る道が今のリポジトリに無い
- 対象: impact.situations[update].can / [rollback].can
- 種類: 言い切り
- 指摘: `update` は「ST59 を含む版に入れ替えると、今の設定のファイルをそのまま読み」と書く。`basis: derived` で、根拠は D3（鍵と値が文字列の JSON であること）だけ。入れ替える道の実在は確かめていない。
  実物では、既に入っているインストールは版を替えられない。`crates/installer/src/commands.rs:215-217`・`crates/installer/src/main.rs:772`・`docs/install.md:33` が「版を替える口はまだ無いので、何も替えずに止まる」と言っている。`commands.rs:710` はその口を ST57・ST58 のものとしている。
  つまり、今インストールした形のサーバは、版を替える口ができるまで ST59 の節を得られない。`rollback` も同じ。「入れ替える・戻す」はこの Story の外の Story に依る。`cannot: この分析では見つけていない` は、見つけられる場所に書いてある。
- 直し方の案: `update` と `rollback` の `can` を「設定のファイルの読み方は変わらない（D3）。入れ替える口自体は ST57・ST58 まで無い」に直し、IMP-01 に「インストールした形の人にも、版を替える口ができるまでは届かない」を足す。
- 処置: fixed: situations の update を「新しく入れたインストールは変えられる / 今のインストールへ入れ替える口はまだ無い（ST57・ST58）」に、rollback を「仕様から言えること。戻す口は確かめていない」に直し、install.md:32 を根拠 E16 に足した

## R3 — D1（インストーラの一時の書き）の evidence が主張を支えず、手当ての実在が「予定」だけになっている
- 対象: impact.dependencies[D1].evidence=E1 / holds / impact.findings[IMP-03]
- 種類: 根拠
- 指摘: E1（`commands.rs:461`）の quote は「中のファイルの持ち主もインストーラなので、ACL の変更が中へ届く」という前提の文そのもの。D1 の `assumes` と同じ文で、「届かないかもしれない」側の根拠にはなっていない。
  同じコメントの次の 3 行（`:463-465`）は、持ち主がサービスのアカウントのファイルには ACL の変更が届かないと明言している。これは、サービスが付け替えで作った `server.json` に当たりうるという直接の根拠なので、こちらを引くべき。
  一方で `:570-608` の `takeown /A /R`（特権を自分で有効にして持ち主を Administrators に戻す）は、届く側の材料になる。IMP-03 の unknowns に takeown の名前はあるが、`derived` はどちらの側に倒れるかを言っていない。
  `holds: unknown` は妥当。ただし evidence は `:463-465` と `:570-608` の両方に替えるのがよい。
  閉じ方については、Task 7.1 と install-check.sh の通しはまだ存在しない。`.github/workflows/ci.yml:483` の `install-windows` job は実在する。「merge の前に CI の緑を見る」という閉じ方は、通しが書かれて緑になって初めて成り立つ。いまの `safe_now` に当たる文は「予定」である。
- 直し方の案: E1 を `commands.rs:463-465` と `:581-608` に替え、IMP-03 の閉じ方に「通し（Task 7.1）が緑になるまで merge しない」を条件として明記する。
- 処置: fixed: E1 を commands.rs:464（持ち主がサービスのファイルへは ACL の変更が届かない）に替え、takeown の行を E14 に足した。IMP-03 の unknowns に「Task 7.1 の通しはまだ予定」を足した

## R4 — IMP-01 の重大度と扱い。本人の運用は推測でなく、手元の資料から絞れる
- 対象: impact.findings[IMP-01]、dependencies[D7]、scope
- 種類: 重大度
- 指摘: `basis: unconfirmed` のまま `medium` で、確かめ方も「本人に聞く」だけになっている。しかし `docs/network.md:35-45` は、いまの手順を `.env` の `API_TOKEN` / `WEB_PASSWORD` で書き、`tools/ports.sh` の既定も WSL の開発の形である。
  `proposal.md` の Why は「サーバをサービスにした後」を前提にしているが、本人が実際にインストールした形で運用しているかは、どの資料にも出てこない。
  .env の形なら、この Story の全体が本人の手元では何も変わらない（節は見えるが 409）。IMP-01 は個別の残りではなく、Story の効き目の前提である。
  それなのに `overview` は先頭で「Windows にインストールした形のサーバなら…変えられるようになる」と限定を付けており、書き方は誠実。重大度の問題というより、`high` にするかどうかを unknowns に書いていないことが足りない。
  また、画面の 409 の文言が「.env を書き換えて起動し直してください」だけで、携帯を落とした直後に本人が最初に見る文になる。IMP-02 の「網からノードを外す」と同様に、先にやることを 1 行足す案は IMP-01 の options に無い。
- 直し方の案: unknowns に「本人が .env の形なら、この Story は本人の携帯の締め出しには何も効かない」を足し、本人の運用の確認をレビューの前に置く。options に「409 の文言に手順書へのリンクを足す」を足す。
- 処置: fixed: IMP-01 の unknowns に「.env の形のままなら本人の手元では効かない」と手順書が .env の形を書いていることを足した。重大度は medium のまま（kept の理由: 配る形はインストールした形で、移れば効く。止めるのは移るまでの間で、今どおりの .env の手順が残る）

## R5 — IMP-02 の読みは概ね正しい。ただし「本人もログインできない」は本人の窓の外の効果で、閉じ方の 2 択に抜けがある
- 対象: impact.findings[IMP-02].text / options、impact.rules[R2]
- 種類: 根拠
- 指摘: `web_session.rs:257-290` と `:704-744` を読むと、数えは 1 つの `VecDeque` で、`throttled()` は誰の失敗かを区別せず、ログインにも同じ道で効く。`derived`・`medium` はこのコードから言える。
  ただし、記述の中に確かめていない断定が 2 つある。
  1. 「拾った人が間違え続ける」手段は、拾った携帯の印で新しい口へ送る場合に限らない。ログインの口を叩くだけでも同じ数えを進められ、網に入っていれば印が無くても可能。つまり、ログインしたままの携帯は条件ではない。この点は既存の挙動で、ST59 が増やす新しい攻撃面ではない。増えるのは、「締め出しの口」が止められること。
  2. 「網からノードを外せば 60 秒で窓が空く」は、窓が `THROTTLE_WINDOW` = 60 秒（`:47`）だからで、外したあとも既に溜まった失敗が 60 秒残る。つまり締め出しの完了までに最大 60 秒の待ちが入る。この待ちは本人には分からない。
  options の 2 つ目（画面の案内）は、携帯のブラウザを使っている拾った人にも見える位置に置くと害になるので、本人の画面の設定の節に限ること。
- 直し方の案: text から「ログインしたままの携帯で」を外して「網に入っている誰かが」にし、「外した後も最大 60 秒は 429」を IMP-02 の残りに書く。
- 処置: fixed: IMP-02 の text を「ログインの口は前から同じ数え。ST59 は変える口もそこに入れる」「ノードを外した後も最大 60 秒は断られる」に直し、scope.residual に 60 秒を足した

## R6 — 依存の見落とし。`+1` のポートは、台本の注意書きどおり 2 つの縦串と panic の確かめも使っている
- 対象: impact.dependencies（T4 / D8 の台本）、design.md D8
- 種類: 依存の見落とし
- 指摘: design D8 は `tools/settings-server.sh` の待ち受けを `ASHIATO_HTTP_PORT + 1` とし、「縦串と e2e は同時に走らない」で済ませている。実物は `tools/ports.sh` の説明、`tools/check-panic-log.sh:9`、`tools/smoke.d/202610241500_st29-satellite-credentials.sh:5`、`tools/smoke.d/202610212100_st13-archive-hands-off.sh:42` が同じ `+1` を使っている。
  縦串どうしは同時に走らないという前提は、既存の 2 本が既に頼っている。3 本目に e2e を足すなら、CI の job 内の並びで被らないことを確かめる必要がある。
  `crates/server/src/net_guard.rs:37` の開発の既定が `127.0.0.1:18788` で、main の checkout では `+1` がこの既定と重なる。
  `dependencies` に台本の待ち受けの被りが無い。設計は反転条件で「被ると分かったら台帳に名前を足す」としているが、被る相手が既に 3 つ見えているので、`unknown` でなく先に確かめられる。
- 直し方の案: dependencies に `+1` の他の使い手 3 つを足し、e2e と同じ job で走る順序を確かめるか、最初から ports.sh の台帳に名前を足す。
- 処置: fixed: 依存 D10（番号 + 1 を使う台本と開発の形の既定 18788）と IMP-05（low）を足し、check-panic-log.sh:9・net_guard.rs:37 を根拠にした

## R7 — 新しい帰結。画面で変えた後の `repair` と走らせ直しの記述が、実物の分岐より単純
- 対象: impact.situations[recovery]、dependencies[D1]
- 種類: 新しい帰結
- 指摘: `recovery` と D1 は、`password` の通しだけを確かめるとしている。しかし `main.rs:975-1000` では、走らせ直しの入口（`decide_install`）が `read_settings_unprivileged(&settings_path)` で `WEB_PASSWORD_VERIFIER` の有無を見る。この読みは、持ち主が違う管理者のときは `None` を返す（`:628`）。
  ST59 の後は `server.json` の持ち主がサービスのアカウントに変わるので、走らせ直しの管理者が本人以外のとき、`verifier_present` が false になりうる。そのとき `decide_install` が `RepairRequired` を返すかどうかは `state.rs` の分岐次第で、分析は見ていない。
  `repair` は「設定のファイルが無いこと」が条件なので、本人の `server.json` があれば使えない（`main.rs` の `repair` が `state::repair_allowed` に渡す条件。`crates/installer/src/main.rs` の `fn repair` の冒頭）。つまり、止まったあとの出口が無い組が、理屈の上では作れる。
  これは推測で、`unknowns` に入れて Task 7.1 の通しに「持ち主を変えた後に別の管理者で走らせ直す」を足すかの判断になる。
- 直し方の案: dependencies に「走らせ直しの入口（`main.rs:975-1000`）が `server.json` の持ち主に依って読めるか」を `unknown` で足し、通しの項目に入れる。
- 処置: fixed: IMP-03 の unknowns に、走らせ直しの段の判定（read_settings_unprivileged）が持ち主の変わった server.json を読めずに repair を求める組があるか（推測）を足し、E15 を根拠にした。coverage の not_examined を直した


## R8 — IMP-04 を high に上げたが、閉じ方の材料（if_accepted）が「いま進めて安全」を言っていない
- 対象: impact.findings[IMP-04].if_accepted / options / unknowns
- 種類: 閉じ方
- 指摘: if_accepted は residual・scope・recheck（owner と due）だけで、`release_ok_why` が無い。recheck の `due: ST58 の上流の合意まで` は予定を書いただけで、ST59 を配ってよい理由になっていない。
  手当ては「2 つを同時に行わない」という本人の注意のみ。取り消した値が黙って戻る帰結（E2・E3: `main.rs:1421-1431`）を抑える実在の仕組みは無い。
  design.md D2 は「インストーラの password はサーバのサービスを止めてから書くので、サーバとインストーラが同時に書くことは無い」と書いている。これは読み → 入力待ち → 停止の順を見ておらず、IMP-04 と食い違ったまま残っている。受け入れて進むなら D2 のこの行も直す必要がある。
  直す側（options 1）は `read_settings` を停止の後へ移し、1 つの鍵だけ置き換える小さな変更で、Task にも入れられる。受け入れるより安く、受け入れる理由が立たない。
  high にした根拠は妥当（資格情報の復活・本人が気づけない・配った後は気づけない）。ただ unknowns が「同時の場面があるか」の 1 つだけで、「入力待ちが何分続きうるか」「画面から変える人が同じ本人か」が無い。
- 直し方の案: 受け入れるなら if_accepted に `release_ok_why`（何が帰結を抑えるか）を足し、D2 の「同時に書くことは無い」を直す。そうでなければ options 1 を Task に入れて reflected にする。
- 処置: fixed: IMP-04 の unknowns に入力待ちの長さと、変える人が同じ本人かの 2 つを足した。design D2 の食い違いは GO 資料の open（未解決）に出した。kept: release_ok_why は書かない（いま配ってよいと言える手当てが無いので「受け入れる（リリースしてよい）」を出さない）。reflected にするには仕様と design を直す必要があり、reading の段は仕様を書き換えないので、IMP-04 の「仕様を直す」を本人に問う
