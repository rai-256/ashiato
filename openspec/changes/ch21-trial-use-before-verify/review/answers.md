spec-digest: 35ef00d80ffe328da3ba99c81336866432c8a75d402a185053053eba43254f15
- Q8: 写った
- Q11: 写った
- Q-G1: 写った
- Q-G2: 写った
答え以外の変更: あり（Q-G2 の道の中身を仮の D35 に置いた: 利用者と網の名前空間を分けた箱（unshare）でエミュレータと専用の adb のサーバを起こし、サーバへは Unix のソケットの橋（socat）、adb は nsenter で箱に入って呼び、エミュレータの網の引数（restrict=on）には頼らない —— spec の携帯の Requirement の「箱の中の adb のサーバ」「<state> に置いた橋（Unix のソケット）でだけ」「片付けで橋と箱を止める」、Scenario「エミュレータと adb は、網を持たない箱の中で動く」の偽の unshare・nsenter、tasks の Global Constraints（偽の unshare・nsenter・socat）・9.1 の箱・橋・nsenter の段と片付け・10.1 の nsenter での確かめ、proposal の携帯の相手の行。D22（仮）の emulator-start.sh の関数と adb を箱の中で使う文と、install_on_emulators が席のエミュレータに入らない文（Risks の「手で呼んだ席のエミュレータ」を消した）。D26（仮）の携帯の対照の行（箱の中の橋の待ち受け）。Risks の「携帯の相手は箱の中の adb reverse が通るまで回らない」）
答え以外の変更の決定: D22 D26 D35
