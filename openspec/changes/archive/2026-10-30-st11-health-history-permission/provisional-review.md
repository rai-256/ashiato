## D15
- 判定: 確か
- 根拠:
  1. 失わない側（忘れて作り直す）で失うもの無し —— `forget()` は端末の `health_permission` の `last_state` の鍵を消すだけ（`HealthPermissionRecord.kt:119`）。
     これは観直せば同じ値が戻る導出値で（`HealthState.of(reading, autoRequested())`、`auto_requested` の印は別の鍵 `ASKED_HEALTH_PERMISSIONS` で消さない）、S-01 の行・未送信の記録・外へ出たものには触れない。
     呼ぶのは記録が置き場から消えた**後**（`Retention.kt:56` の注記・`:101-104` は確定した `confirmed` だけを渡す）で、`LocationService.kt:662-666` は捨てた中に `HEALTH_LOGICAL_SOURCE` があるときだけ呼ぶ。
     `Retention(` の生成は `LocationService.kt:650` の 1 か所だけ（`grep -rn 'Retention(\|dropHead\|dropUntilBytes' collector-android/app/src/main`）なので、捨てる経路の取りこぼしは無い。
     manifest に `android:process` は無く（grep 0 件）、画面（`MainActivity.kt:94,111`）とサービス（`LocationService.kt:1050-1051`）は同じプロセスの同じ `HEALTH_PREFS` を見るので、画面側の recorder にも忘れたことが見える。
     画面の控え（`HealthPendingSlot`）が残るのは受け渡し口に預けてから未送信に書けるまでの間だけで、その間の記録はまだ未送信に無いので捨てられない。
     作り直した記録の `device_time` が後ろへずれるのは捨てたこと（ST04 の保持の上限。この決定の前からある）によるもので、忘れない側でも戻らない。D15 の選択で新たに `uncaptured` / `discarded` / `exported` / `rewrite-all` は起きない
  2. cost は高くつかない —— 1 行の呼び出しと 1 本の試験（`HealthNotificationTest.kt:181-195`）。範囲は ST11 が足した置き場の中で、要件の本文は変えない。
     重複は古い記録 S1 だけ捨てられて新しい S2 が未送信に残っているとき、S2 と同じ状態の記録がもう 1 件増える場合に起きうる。台帳の cost「同じ状態の記録が 1 件増えうる」と合い、D2 の「作り直す側に倒れる」（`HealthPermissionRecord.kt:111`）と同じ向き
  3. `follows` は null（該当なし）
  4. `rests_on: [D2]` は合う（比べる相手の「最後の状態」は D2 の判定）。作り直す時は D9 の契機（開く・6 時間ごと。`design.md:266`）で決まる。ただし D9 が変わっても、作り直すのが遅れるだけで、忘れる側を選ぶことは変わらない。D14（置き場と自動バックアップ）、D1（名前。判定は定数 `HEALTH_LOGICAL_SOURCE` を参照）も D15 の向きを変えない。挙がっていない土台は無い
