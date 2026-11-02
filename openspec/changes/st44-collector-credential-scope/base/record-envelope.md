# record-envelope の書いた時点の正典（spec_merge.py snapshot。archive の 3-way の base。手で直さない）

### Requirement: すべての API 要求は資格情報を要求する

THE SYSTEM SHALL すべての API 要求に、呼び出し元を識別できる資格情報を要求する。
THE SYSTEM SHALL 資格情報を欠くか一致しない要求を 401 で拒否する。
THE SYSTEM SHALL 資格情報の比較を、一致した長さから内容が推測されない方法で行う。

導出元: PERM-10。
PERM-7（外部からの到達を Tailscale 網内に限る）は**網の外を止めるだけ**で、
**同じ PC の中から呼ぶものを何も止めない**。第三者製のプラグインは同じ PC で動く。

#### Scenario: 資格情報が無いと拒否される

- **WHEN** 資格情報を付けずに取り込み口または読み出し口を呼ぶ
- **THEN** 401 で拒否される

#### Scenario: 同じ PC の別プロセスからでも拒否される

- **WHEN** 同じ PC 上の別プロセスが資格情報を付けずに呼ぶ
- **THEN** 401 で拒否される
