# ST59 の前提（外の形式・振る舞い）

この Story の決定のうち、外の形式（Windows のファイルの権限・置き換え）と既存の仕組みに依るもの。
失う側を持つ論点（読み出しの記録に残すか・古い共有の値の猶予・ログインを残すか）は、どれも失わない側に決まっている
（読み出しの記録に残すは聞かない既定、猶予なしは FR-128、ログインを切るは正典の data-sensitivity）。人間に問う 3 問は失うものの無い B で、
どの前提にも `loss` のある決定は載っていない。そのため確かさ 2 のまま問いを立て、下流の試験（3 行目はサーバの試験、4 行目は収集アプリの既存の試験を走らせる Task 1.3、1・2・5 行目は CI の Windows runner の Task 7.1）で 4 に上げる。Task 1.3 の既存試験が通ったため、4 行目は確かさ 4 とする。
5 行目は外れると合言葉を忘れたときの戻り道（インストーラの `password`）を失うので、CI で確かさ 4 に上げてから merge する（design D5。review/spec.md R10）。

| 前提 | 何が依るか | 確かめ方 | 結果 | 出所 | 確かさ |
|---|---|---|---|---|---|
| サーバのサービスのアカウント（`NT SERVICE\ashiato-server`）は `settings\server.json` を書き換えられる | 変えた値を、サービスを起こし直した後も持つ（設定のファイルに書き戻す） | インストーラが状態の置き場の根に付ける ACL を読む | 根に `ashiato-server:(OI)(CI)M`（継承で `settings\` にも及ぶ）。`owner.json` だけは別に管理者と SYSTEM に絞る | 既存のコード `crates/installer/src/commands.rs:425-440`（`acl_commands`）・`crates/installer/src/secrets.rs:84` | 2 |
| 同じフォルダの中で一時のファイルを書いて名前を付け替えると、Windows でも既存の `server.json` を 1 回で置き換える（途中の半端なファイルが残らない） | 書き戻しの途中で落ちてもサーバが起きなくならない | Rust の `std::fs::rename` の文書（Windows では `MoveFileExW` に `MOVEFILE_REPLACE_EXISTING`） | 置き換える | 公式文書 https://doc.rust-lang.org/std/fs/fn.rename.html | 2 |
| ログインの印は画面の合言葉の照らす値（verifier）に結び付いていて、照らす値を変えると、それまでの印は全部通らなくなる | 合言葉を変えたら、拾われた携帯のログインも切れる | 印の照合のコードを読む | `core.web_session.secret_tag` が `secret_tag_for_verifier(鍵, verifier)` と一致する印だけ通す。インストーラの `password` の口（決め直し）も同じ仕組みで全部の印を失効させている | 既存のコード `crates/server/src/web_session.rs:243-249`・`:442-458`・`crates/installer/src/main.rs:1417` | 2 |
| 収集アプリは 401 で断られても記録を捨てず、1 時間続くと設定し直しを促す。送り先が同じ設定の文字列を貼り直しても、送り先の保留にはならない | 共有の値を変えた直後から古い値を断っても、記録を失わない | `cargo test -p ashiato-collector-windows st56_runtime_unauthorized_keeps_pending_records` と Android `St56SetupNotifierTest` を実行 | Windows collector の `st56_runtime_unauthorized_keeps_pending_records` と Android `St56SetupNotifierTest` が PASS | `crates/collector-windows/src/runtime.rs:1815`・`collector-android/app/src/test/kotlin/dev/ashiato/collector/St56SetupNotifierTest.kt:75-76` の既存試験 | 4 |
| サーバのサービスのアカウントが付け替えで書き直した `server.json` を、インストーラの `password`・走らせ直し・`repair` が今どおり読み書きできる（ファイルの持ち主がサーバのアカウントに変わっても、一時の書きと `takeown` が届く） | 合言葉を忘れたときの戻り道（インストーラの `password`）が、画面で変えた後も使える | インストーラの一時の書きの命令を読む。実物は CI の Windows でサーバに書かせた後に `password` を走らせる | 一時の書きは「中のファイルの持ち主がインストーラ」を前提に書かれ、`takeown /A /R` で持ち主を戻す。サーバが作ったファイルに届くかは未確認 | 既存のコード `crates/installer/src/commands.rs:462-465`・`:570-608` | 2 |
