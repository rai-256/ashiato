## ADDED Requirements

### Requirement: 確認バッチの準備は、エミュレータにだけ携帯の APK を入れる

THE SYSTEM SHALL 確認バッチの作業ツリーの準備（`tools/verify-prep.sh`）に、debug の鍵の携帯の APK を、`adb devices` が `device` の状態で返す端末のうち、名前が `emulator-` で始まるものにだけ入れさせる。
THE SYSTEM SHALL `emulator-` で始まらない端末（USB や無線で繋いだ実機）には、繋がっていても APK を入れさせない。
THE SYSTEM SHALL エミュレータが 1 台も無いときは、何も入れさせない。そのとき確認の手順書には、エミュレータが無いので入れていないことと、手で入れるときのコマンドを書かせる。
導出元: 扉 34 ★ 2026-11-01（本人の携帯には、最初から正式な鍵の版を入れる）。ST52 の code_verify R30・`docs/audit/2026-11-01-st52-door34-consistency.md` の K1（`docs/handoff/ST53.md` の申し送り）。深掘り Q8（第 1 回の推奨）。
debug の鍵の版が本人の携帯に入ると、正式な鍵の版へ移るときに入れ直しが要り、未送信の記録を失うことがある。

#### Scenario: 実機とエミュレータが繋がっていれば、エミュレータにだけ入れる
- **GIVEN** `adb devices` が実機 `R58N12ABCDE` とエミュレータ `emulator-5554` を、どちらも `device` で返す偽の `adb`
- **WHEN** 確認バッチの準備の端末の選び方を走らせる
- **THEN** `adb -s emulator-5554 install -r` だけが呼ばれ、`R58N12ABCDE` への `install` は呼ばれない

#### Scenario: 実機だけが繋がっていれば、何も入れない
- **GIVEN** `adb devices` が実機 `R58N12ABCDE` だけを `device` で返す偽の `adb`
- **WHEN** 確認バッチの準備の端末の選び方を走らせる
- **THEN** `install` は 1 回も呼ばれず、エミュレータが無いので入れていないことと、手で入れるときのコマンドが示される

#### Scenario: 準備の台本は、端末を選ぶ手順を通して APK を入れる
- **WHEN** 確認バッチの準備の台本を読む
- **THEN** `adb` の `install` は端末の選び方の手順の中にだけあり、`adb devices` の最初の端末へ直に入れる行は無い
