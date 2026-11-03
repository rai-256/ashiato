# impact-review — st55-server-installer

seen: IMP-01 medium, IMP-02 medium, IMP-03 medium, IMP-04 low, IMP-05 low

## R1 — サービスの回復が「終わった」サーバを起こし直すかは、登録の命令だけでは言えない
- 対象: impact.rules[R2].legit（「止まり続けはしない」）、situations[recovery].can（「DB やサーバが異常に終わればサービスの回復が起こし直す」）、IMP-02 の S5（「回復で繰り返す」）
- 種類: 新しい帰結
- 指摘: design.md:104 は `sc.exe failure … actions= restart/…` だけで、`sc.exe failureflag <名前> 1` を書いていない（design.md・tasks.md・spec を `failure` で引いても他に無い）。Windows の回復は、サービスが SERVICE_STOPPED を告げずに落ちたときだけ動く。サーバが自分で「理由をログに書いて終わる」（spec/distribution:70、R2）とき、停止を告げて非 0 で終わる作りだと、failureflag が無ければ回復は動かない。すると、DB の起動が 120 秒より遅い PC や設定を読めない起動で、サーバは次の再起動まで止まったままになる。R2 の legit の括弧書きは根拠のない言い切り。
- 直し方の案: Requirement / Task に `failureflag 1`（または、終わるときは停止を告げずにプロセスを落とす）を明記して Windows の CI で「サーバを非 0 で終わらせると起こし直される」を見るか、R2 / recovery の括弧書きを外す。

- 処置: fixed: IMP-06（medium・推測）を足した。rules[R2].legit・situations[recovery].can・S5 の「回復が起こし直す」の言い切りを外し、IMP-06 を指した。仕様は直さない（reading の段）。直すかは本人が IMP-06 で選ぶ

## R2 — IMP-02: 55400 が動的な範囲にあることの害は、予約だけでなく「DB の起動前に送り側の番号として取られる」もあり、仕様には選ぶ口が無い
- 対象: impact.findings[IMP-02].text / severity / options、rules[R9].legit
- 種類: 重大度
- 指摘: 動的な範囲（49152〜65535）は、Hyper-V の予約に加えて、外向きの接続の送り側の番号にも使われる。起動時に他のプログラムが 55400 を送り側に取れば、DB のサービスは待ち受けられない。これは IMP-02 の text に無い。いっぽう 18700 は範囲の外で影響を受けない。害が起きると、DB は起きられず、サーバは DB に依るので起きない。ログを見ない限り本人は気づけず、番号を変える口もない（R9 のとおり）。これは high の条件（口が無い・自分で気づけない）に当たる形だが、premises C5 は「番号は作り直せば変えられる」とし、起きる確率も確かめていないので、根拠のない high にはならない。medium は妥当。ただし直す費用は今がいちばん小さい。配った後は、設定のファイル・手順書・Tailscale の案内・ST56 の繋ぎ先に 55400 が広がる。options の 1 番目（動的な範囲の外に替える）を「確認バッチまで持ち越す」より先に決めたほうがよい。
- 直し方の案: IMP-02 の text と S5 に送り側の番号の経路を足し、番号を 49152 未満（例 15400 や 5xxx 台。開発の割り当てと被らないこと）に替えるか、持ち越すなら「配ってから番号を替える手順（設定のファイル・サービス・ST56）」を書く。

- 処置: fixed: IMP-02 の text と S5 の occurs_when に送り側の番号の経路と「配る前が一番安い」を足し、持ち越す案に「配った後に番号を替える手順が要る」を添えた。重大度は medium のまま（指摘も妥当とした）

## R3 — D3 の「observed」は、ST44 がまだ実装していない WEB_SESSION_KEY まで見ていない
- 対象: impact.dependencies[D3]（basis: observed, evidence E5・E12）
- 種類: 根拠
- 指摘: D3 の assumes は「`WEB_SESSION_KEY` を鍵とする」前提を含むが、実物の web_session.rs:98-103 の `secret_tag` は鍵が `api_token`（`WEB_SESSION_KEY` は crates 全体で 0 件）。E5 が支えるのは `ashiato-web-session-v1` の文字列だけで、鍵と ST44 の「印の内容は変えない」は E12（ST44 の design。まだ実装されていない仕様）が言うこと。ST44 は archive 前で、下流の実装がこの change の入口に取り込まれるまで、v1/v2 の分岐（design D8）が載る土台は実物に無い。`holds: yes` は、ST44 の実装が仕様どおりに来るという前提つきの `spec` に近い。
- 直し方の案: basis を `spec`（または observed と spec の併記）にして、ST44 の実装が入ったときに `secret_tag` が `WEB_SESSION_KEY` を鍵にしていることを確かめる Task / unknown を足す。

- 処置: fixed: D3 を basis: spec・holds: unknown にし、いまの鍵が API_TOKEN であることを affects に、ST44 の実装を取り込んだ後に secret_tag の鍵を確かめることを verify に書いた

## R4 — D16 / IMP-04 の「いまの経路は main の 18787」は、コード中の注釈を根拠にしている
- 対象: impact.dependencies[D16]（observed, E8）、findings[IMP-04].text（「携帯の収集アプリはまだ入っておらず…見つけていない」）
- 種類: 根拠
- 指摘: E8 は tools/ports.sh の注釈（「スマホから見る tailscale serve の先」）で、実際の `tailscale serve` の設定ではない。また「携帯の収集アプリはまだ入っていない」「PC の収集アプリの自動起動も無い」には evidence も refs も無い。限定つきの書き方（「この分析では見つけていない」）なので言い切りではないが、`observed` と `low` の根拠としては弱い。
- 直し方の案: `basis` を spec / unconfirmed に下げ、「本人の切り替えの前に tailscale serve status と収集アプリの状態を見る」を unknown に足す。重大度の low は、捨てられる記録が無いと確かめてからにする。

- 処置: fixed: D16 を basis: unconfirmed にし、IMP-04 に unknowns（実際の tailscale serve の転送先・収集アプリの状態）を足して、収集アプリの状態を「設計はそうしているが確かめていない」と書き直した。low は、確かめていないと明記したうえで残す

## R5 — IMP-01 の「普通の操作では無くならない」は確かめていない
- 対象: impact.findings[IMP-01].text、rules[R7].legit
- 種類: 言い切り
- 指摘: text は「本人も書けない置き場にあるので、普通の操作では無くならない」と断定するが、unknowns は「普通の利用でこの状態に当たる道があるか」を未確認としていて、矛盾している。置き場の設定のファイル以外にも、ウイルス対策による隔離（not_examined に挙げた）・ディスクの異常・ST58 の戻しで置き場の一部だけを戻す、がこの状態に当たりうる。medium は妥当だが、断定は unknown と食い違う。
- 直し方の案: 「普通の操作では無くならない」を「当たる道を、管理者の操作のほかには見つけていない」に替える。

- 処置: fixed: IMP-01 の「普通の操作では無くならない」を「管理者が消した・一部だけ戻した場合のほかに、この分析では見つけていない」に替え、確かめていない経路を挙げた

## R6 — IMP-03 は妥当。ただし「開く URL を決める」側の選択肢で足りるかを見ていない
- 対象: impact.findings[IMP-03] / dependencies[D4]
- 種類: 根拠
- 指摘: E4 は Chromium の fetch で、`playwright.config.ts` の baseURL が 127.0.0.1 のとき `Secure` の印が残ることを見ている（login-cookie.spec.ts:12-33）。主張（Chromium では確かめた・他は未確認）は E3・E4 から言える。medium でよい（合言葉の入力はできず、記録の損失は無い）。ただし options の 2 番目（localhost に替える）は、Chromium では `localhost` も同じ扱いなので、Firefox の扱いが 127.0.0.1 と `localhost` で違うことを確かめてから選ぶ必要がある（確かめずに URL を替えると、確かめた範囲が変わる）。
- 直し方の案: verify に「playwright の firefox で 127.0.0.1 と localhost の両方」を書く。

- 処置: fixed: IMP-03 の verify を、firefox で 127.0.0.1 と localhost の両方を走らせ、URL を替える案はその結果を見てから選ぶ、に替えた

## R7 — 依存は ST57/ST58 と ST44 の読みに大きな見落としは無い。ST30 の書き側だけ根拠がない
- 対象: impact.dependencies[D7]（holds: yes, basis: spec）
- 種類: 依存の見落とし
- 指摘: D7 は「ST30 の毎日のバックアップがサーバのアカウントで `backups` に書ける」を spec 根拠で yes としている。実物では、`pg_dump` を呼ぶのは migrate_backup.rs だけで、ST30 の実装はまだ無い。バックアップが `pg_dump` を使うなら D2 と同じく PATH の問題（System32 で動くサービスに同梱の pg_dump が見えない）が ST30 側でも起きる。ST30 が `PG_DUMP` / `MIGRATE_BACKUP_DIR` のような設定の鍵を読む前提を、依存に足すとよい。D8（ST57/58）を unknown / guess にしたのは妥当。
- 直し方の案: D7 を `holds: unknown`（basis: unconfirmed）にして、「ST30 のバックアップが `PG_DUMP` とバックアップの置き場を設定のファイルから読める形にする」を verify に書く。
- 処置: fixed: D7 を ST59 だけ（yes）にし、ST30 を D17（holds: unknown・basis: unconfirmed）に分けて、PATH の pg_dump の問題を affects、設定のファイルから読む形にすることを verify に書いた
