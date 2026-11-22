# インストール後の案内

## 入れる

1. 配布 zip を展開し、展開したフォルダの `ashiato-setup.exe` を実行します。
2. Windows SmartScreen が表示されたら、発行元と入手元を確認して「詳細情報」→「実行」を選びます。
3. UAC は 1 回だけ確認します。インストール先は `C:\Program Files\ashiato`、状態の置き場は `C:\ProgramData\ashiato` のままにします。
4. 画面の合言葉を 2 回入力して決めます。合言葉は 16 文字以上で、`change-me` で始めないものにします。入力した文字は画面に表示されません。合言葉を覚えているうちは画面の設定の「画面の合言葉を変える」から変更できます。忘れた場合は今までどおり `ashiato-setup.exe password` を実行して設定し直します。

インストーラは DB サービス `ashiato-db`（55400）とサーバサービス `ashiato-server`（18700）を登録します。サーバ PC では Tailscale の「無人で動かす」を有効にし、サーバ PC 上で `tailscale serve --bg --https=443 http://127.0.0.1:18700` を実行します。画面は Tailscale のアドレスまたは名前の HTTPS URL から開きます。インストール前に状態の置き場に 40 GB 以上の空きを確保してください。

## 更新の確認

更新の確認は有効な状態でインストールされます。更新係は `ashiato-updater` サービスとして動き、配布元から版の一覧だけを取得して、利用できる新しい版を確認します。配布元へ送るものは版の一覧の取得だけです。接続元の IP アドレスと時刻は配布元側の記録に見えるため、運用上必要なときは更新係の確かめた記録と合わせて確認します。

更新の確認を切る、または戻すときは、管理者の端末で次を実行します。

```bat
ashiato-setup.exe update-check off
ashiato-setup.exe update-check on
```

更新係の設定と確かめた記録は `C:\ProgramData\ashiato-updater` に置かれます。確かめた記録を確認するときは、この置き場の `status.json` と `checks.jsonl`、および `logs\` を対象にします。更新係を手で消す必要がある場合は、先に `ashiato-setup.exe update-check off` を実行してから、管理者の端末で次を実行します。

```bat
sc.exe delete ashiato-updater
```

0.1.0（commit `de49710d1`）の更新係は、配布元との最初の HTTPS の接続で異常終了し、Windows の回復動作で 10 秒ごとに再起動を繰り返します（イベント 7031。手で `ashiato-updater.exe once` を走らせると `WebPki is disabled` と出ます）。この版を入れている場合は、先に `ashiato-setup.exe update-check off` で止め、0.1.1 以降の zip の `ashiato-setup.exe` を管理者として実行して更新してから `ashiato-setup.exe update-check on` で戻します。0.1.0 の更新係は自分では直った版を取りに行けません。版番号が 0.1.0 のまま中身だけ違う zip（PR #329 の CI の成果物など）は、インストーラが「別の版が入っています」で止めるので使えません。

サービスを手で消した後は、`update-check on` だけではサービスを再登録できません。更新係を戻すには、同じ版のインストーラを走らせ直して不足している更新係を足してから、`ashiato-setup.exe update-check on` を実行します。`repair` は DB や秘密の復旧用で、更新係の再登録には使いません。

## サーバの更新

サーバを更新するときは、新しい版の配布 zip を展開し、その zip に同梱された
`ashiato-setup.exe` を管理者として実行します。更新の間は `ashiato-server` を停止するため、
画面と収集アプリからサーバへ接続できません。更新が終わると新しい版でサービスを起こします。

移行に失敗した場合は、インストーラがその回の開始前に作った
`C:\ProgramData\ashiato\update\backups\pre-migrate\<run_id>\` から DB と設定を戻し、
前の版でサービスを起こします。結果は `C:\ProgramData\ashiato\update\outcomes.jsonl` に
`rolled_back` として残ります。戻したことはインストーラの窓と画面の帯に表示され、次の更新が
通るまで帯に残ります。

更新の途中で電源断などにより止まった場合は、まず新しい版の `ashiato-setup.exe` をもう一度
実行してください。必要なら管理者の端末で次を実行して、前の版に戻してからやり直せます。

```bat
ashiato-setup.exe rollback
```

退避は直前の 1 回分だけ残ります。戻した後に更新をやり直すときも、更新の開始時に作られた
退避を使います。インストーラに同梱された PostgreSQL の大きい版（major version）が、既に
入っている版と違う配布物では更新を止めます。DB の変換を伴うため、同じ大きい版の配布物を
使ってください。

更新の結果を記録する印と `rolled_back` の行は ST58 より前の版にはありません。ST58 より前の
版のインストーラはこの印を読まないため、古い版の `ashiato-setup.exe` で新しい版の更新や
戻しを行わないでください。

## 消す

通常の削除は管理者の端末で `ashiato-setup.exe uninstall` を実行します。記録を消す前に `C:\ProgramData\ashiato\backups` の退避を確認してください。記録を手で消す場合は、管理者として所有権を取り、対象を限定して次を実行します。

```bat
takeown /r /f "C:\ProgramData\ashiato"
icacls "C:\ProgramData\ashiato" /grant Administrators:F /t
```

確認してから `C:\ProgramData\ashiato` を削除します。`takeown /r` と `icacls` は指定したフォルダ以下を対象にするため、パスを変更せず、記録を残す必要がないことを確認してから使ってください。

## 秘密が欠けて止まったとき

DB の置き場は残っているのに設定や秘密が欠けてサーバが起動しない場合だけ、管理者の端末で `ashiato-setup.exe repair` を実行します。`repair` は記録、移行の履歴、原本、書き出しを変更せず、DB の役割の合言葉、`API_TOKEN`、`WEB_SESSION_KEY` と画面の合言葉を作り直します。設定が正常にある場合や DB 自体が無い場合は `repair` を使わず、走らせ直しか `password` を使います。

`repair` は作り直した値を画面に出しません。収集アプリへ入れる新しい `API_TOKEN` は、インストールした本人（Windows の利用者）が `C:\ProgramData\ashiato\settings\server.json` を開いて `API_TOKEN` の値を読みます（このファイルは本人とサーバのサービスだけが読めます。本人は読めますが書けません）。メモ帳で開いて値を写し、ファイルは保存せずに閉じてください。

`repair` の後は新しい資格情報になるため、収集アプリ（携帯と PC）の接続先の設定で、上の `API_TOKEN` に更新して繋ぎ直します。値そのものを記録、ログ、チャット、コマンド引数へ書かないでください。

インストールが「DB の役割の完了記録がありません」で止まったときは、DB の役割を作り終えた記録が無い DB があります。記録が入っているかもしれないので、インストーラは DB を消しません。管理者の端末で `ashiato-setup.exe repair` を実行して DB の役割を作り直します（記録には触りません）。インストーラが自分で `initdb` を始めて役割を作る前に落ちた DB だけは、走らせ直しが自動で作り直します。

インストールが「この zip と違う … があります（別の版が入っています）」で止まったときは、プログラムの置き場に別の版が入っています。版を替える口はまだ無いので、インストーラは何も替えずに止まります。入っている版の `ashiato-setup.exe`（`C:\Program Files\ashiato\ashiato-setup.exe`）で走らせ直してください。

## 残るリスク

- 管理者は所有権を取れば状態の置き場を読めます。管理者権限を持つ利用者を必要最小限にし、端末を共有しないでください。
- サーバが停止してから `repair` を実行するまでの間、携帯側に保持できる未送信データ量には上限があります。復旧まで長引く場合は収集アプリを停止し、空き容量と未送信状態を確認してください。
