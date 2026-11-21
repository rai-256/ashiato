# distribution の書いた時点の正典（spec_merge.py snapshot。archive の 3-way の base。手で直さない）

### Requirement: 更新を確かめる問い合わせは、版の一覧を取る決まった形だけを配布元へ送る

THE SYSTEM SHALL 3 つの部品の更新の問い合わせを、方法 `GET`・本文なしにし、道を次の 2 つに限る: 開発版でない部品は `/repos/rai-256/ashiato/releases?per_page=20`、開発版の部品は `/repos/rai-256/ashiato/releases/tags/nightly`。
THE SYSTEM SHALL 問い合わせの見出しを、`Host`・`User-Agent`・`Accept`・`Accept-Encoding`・`Connection` の 5 つの名前の内側に限り、`User-Agent` の値を `ashiato-update-check` に、`Accept` の値を `application/vnd.github+json` に固定する。
THE SYSTEM SHALL 問い合わせのどこ（道・見出し・本文）にも、自分の版番号・端末の機種・OS の版・端末や利用者の識別子・資格情報・記録を入れない。
THE SYSTEM SHALL 転送（3xx）は同じ `api.github.com` の中のものだけを追い、ほかの宛先への転送は「確かめられない」として扱う。
THE SYSTEM SHALL 暗号化した接続の証明書を OS の持つ根で確かめ、更新係と PC の収集アプリに根の一覧の部品を同梱しない。
THE SYSTEM SHALL 宛先を組み立てのときの定数にし、配る組み立て（サーバの更新係・PC の収集アプリの配る zip、携帯の配る APK）には宛先を実行時に変える口を持たせない。
THE SYSTEM SHALL 宛先を差し替える口を試験の組み立てにだけ持たせ、試験の組み立ては差し替えの宛先が無ければ確かめない（自動の試験は本物の配布元に問い合わせない。配る組み立てを動かす CI は、外への接続を塞いで走らせる）。

導出元: FR-116（機械の検査で固定する。要件の Q7）。聞かない既定 C2・C8。premises P1〜P3・P10。
GitHub の API は `User-Agent` の無い問い合わせを断る（P3）ので、見出しを 1 つは送る。`Connection` は Android の通信の部品が自分で足すことがある（design D13（仮）。聞かない既定 C2 の 4 つに足した）。
自分の版は送らず、取った一覧を手元で選ぶ。配布元から見分けられるのは開発版かどうか（道の違い）だけで、開発版の印 `nightly` は公開されている。

#### Scenario: 更新係の問い合わせは版の一覧の取得だけである

- **GIVEN** 宛先を手元の受け手に差し替えた試験の組み立ての更新係（版 `0.4.0`）
- **WHEN** 更新係が 1 回確かめ、受け手が受けた問い合わせを全部読む
- **THEN** 問い合わせは 1 件で、方法は `GET`、道は `/repos/rai-256/ashiato/releases?per_page=20`、本文は 0 バイト、見出しの名前は許した 5 つの内側、`User-Agent` は `ashiato-update-check`、`Accept` は `application/vnd.github+json` で、どの見出しの値にも `0.4.0` が無い

#### Scenario: PC の収集アプリの問い合わせは版の一覧の取得だけである

- **GIVEN** 宛先を手元の受け手に差し替えた試験の組み立ての PC の収集アプリ（版 `0.4.0`・端末の識別子 `pc-0123456789ab`）
- **WHEN** PC の収集アプリが 1 回確かめ、受け手が受けた問い合わせを全部読む
- **THEN** 問い合わせは 1 件で、方法・道・本文・見出しの名前と値は更新係と同じ決まりのとおりで、どの見出しの値にも `0.4.0` と `pc-0123456789ab` と資格情報が無い

#### Scenario: 携帯の問い合わせは版の一覧の取得だけである

- **GIVEN** 宛先を端末の中の受け手に差し替えた試験の組み立ての携帯の収集アプリ（本物の Android の上）
- **WHEN** 携帯の収集アプリが 1 回確かめ、受け手が受けた問い合わせを全部読む
- **THEN** 問い合わせは 1 件で、方法・道・本文・見出しの名前と値は更新係と同じ決まりのとおりで、どの見出しの値にも端末の機種名（`Build.MODEL`）・OS の版（`Build.VERSION.RELEASE`）・アプリの版・資格情報が無い

#### Scenario: 更新係と PC の収集アプリは根の一覧を同梱しない

- **WHEN** 更新係と PC の収集アプリを組み立てる部品の一覧を、同梱の根の一覧の部品（一覧は検査の側が持つ）と突き合わせる
- **THEN** 一致する部品は 0 件である

#### Scenario: 開発版の部品は印 nightly の 1 件だけを取る

- **GIVEN** 宛先を手元の受け手に差し替えた、版 `0.4.0-dev.20261102` の更新係
- **WHEN** 更新係が 1 回確かめる
- **THEN** 受け手が受けた道は `/repos/rai-256/ashiato/releases/tags/nightly` だけである

#### Scenario: 別の宛先への転送は追わない

- **GIVEN** 手元の受け手が `301` で `api.github.com` でない宛先へ転送を返す
- **WHEN** 更新係が確かめる
- **THEN** 更新係は転送先へ問い合わせず、その回を「確かめられない」（理由: 読めない答え）として記録する

#### Scenario: 配る組み立ては宛先を変える口を持たない

- **WHEN** 配る形でサーバの Windows の配布物と PC の収集アプリの配布物を組み、`ashiato-updater.exe` と `ashiato-collector-windows.exe` の中の文字列を読む
- **THEN** 宛先を差し替える口の名前（`ASHIATO_UPDATER_TEST_`・`ASHIATO_COLLECTOR_TEST_UPDATE_URL`）が無く、宛先 `https://api.github.com/repos/rai-256/ashiato/releases` がある

#### Scenario: 携帯の配る APK の宛先は配布元である

- **WHEN** 配る形で携帯の APK を組み、組み立ての定数の更新の宛先を読む
- **THEN** 宛先は `https://api.github.com/repos/rai-256/ashiato/releases` で、配る組み立てに宛先の差し替え（`-P`）を渡すと組み立ては 0 でない終了コードで落ちる

#### Scenario: 試験の組み立ては差し替えの宛先が無ければ確かめない

- **GIVEN** 宛先を差し替えていない試験の組み立ての PC の収集アプリ
- **WHEN** 確かめる時を迎える
- **THEN** 外への問い合わせは 0 件で、確かめた記録に「宛先なし」が 1 行ある
