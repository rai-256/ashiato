// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-ashiato-play-services
//! `ureq` の係を組み立てる唯一の口。**証明書の根を OS のものに決める**。
//!
//! `ureq` 3 の既定の根は `RootCerts::WebPki`（同梱の根の一覧）だが、この crate はその機能
//! （`rustls-webpki-roots`）を入れていない（spec distribution「根の一覧を同梱しない」・ST57 D12）。
//! 根を指定しないまま係を作ると、**最初の HTTPS の握手で panic する**（`WebPki is disabled`）。
//! 0.1.0 はこれで、HTTPS の送り先・配布元に繋いだ途端に落ちた（2026-11-19 実機）。
//!
//! 係は必ずここから作る。`clippy.toml` の `disallowed-methods` が、ほかの場所での
//! `ureq::Agent::config_builder` などを落とす。

/// 根を OS のもの（`RootCerts::PlatformVerifier`）にした係の組み立て。上限などは呼ぶ側が足す。
#[allow(clippy::disallowed_methods)] // ここが唯一の口
pub fn agent_builder() -> ureq::config::ConfigBuilder<ureq::typestate::AgentScope> {
    ureq::Agent::config_builder().tls_config(
        ureq::tls::TlsConfig::builder()
            .root_certs(ureq::tls::RootCerts::PlatformVerifier)
            .build(),
    )
}

/// 試験で使う: 手元の受け口（`https://127.0.0.1:<port>`）を `call` に渡して 1 回繋がせ、
/// `call` の返した「誤りだったか」と、受け口が読んだ最初の数バイトを返す。
/// 受け口は TLS を話さないので要求は失敗するが、**panic せずに誤りとして返る**ことと、
/// 握手の最初の記録（ClientHello。`0x16 0x03`）が実際に送られたことを確かめられる。
#[cfg(test)]
pub(crate) fn handshake_against_loopback(call: impl FnOnce(String) -> bool) -> (bool, Vec<u8>) {
    use std::io::Read as _;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let peer = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(10)))
            .expect("timeout");
        let mut head = [0u8; 5];
        let n = stream.read(&mut head).unwrap_or(0);
        head[..n].to_vec()
        // ここで閉じる。相手は握手の途中で切られる
    });
    let is_err = call(format!("https://127.0.0.1:{port}"));
    let head = peer.join().expect("peer");
    (is_err, head)
}

/// 試験で使う: [`handshake_against_loopback`] の結果が「握手を始め、panic せず誤りで返った」こと。
#[cfg(test)]
pub(crate) fn assert_handshake_started((is_err, head): (bool, Vec<u8>)) {
    assert!(is_err, "TLS を話さない相手への要求が成功した");
    assert_eq!(
        head.first(),
        Some(&0x16),
        "握手の記録が届いていない: {head:02x?}"
    );
    assert_eq!(head.get(1), Some(&0x03), "TLS の版の印が無い: {head:02x?}");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 根を指定した係は HTTPS の握手を始め、相手が切っても panic せずに誤りを返す（2026-11-19 の 0.1.0 の abort）。
    /// 根の指定を外すと、ここで `WebPki is disabled` の panic になって落ちる。
    #[test]
    fn tls_handshake_starts_without_panic() {
        let agent = agent_builder()
            .timeout_global(Some(std::time::Duration::from_secs(10)))
            .build()
            .new_agent();
        assert_handshake_started(handshake_against_loopback(|base| {
            agent.get(format!("{base}/")).call().is_err()
        }));
    }

    /// 送る係（`sender::agent`）も同じ。送り先に `https://` を入れても落ちない。
    #[test]
    fn tls_sender_agent_handshake_starts_without_panic() {
        let agent = crate::sender::agent();
        assert_handshake_started(handshake_against_loopback(|base| {
            agent.post(format!("{base}/api/ingest")).send("{}").is_err()
        }));
    }

    /// 本物の配布元（`api.github.com`）と握手を終え、**OS の根で証明書を確かめられる**。
    /// 網に出るので既定では走らせない。CI の Windows の job（`collector-windows-runtime`）が `--ignored` で走らせる。
    #[test]
    #[ignore = "網に出る。CI の Windows の job が --ignored で走らせる"]
    fn tls_online_handshake_verifies_with_os_roots() {
        let agent = agent_builder()
            .http_status_as_error(false)
            .timeout_global(Some(std::time::Duration::from_secs(30)))
            .build()
            .new_agent();
        let response = agent
            .get("https://api.github.com/")
            .header("User-Agent", ashiato_update_check::UPDATE_CHECK_USER_AGENT)
            .call()
            .expect("api.github.com と握手できない（根の確かめに失敗した）");
        assert!(response.status().as_u16() < 500, "{}", response.status());
    }
}
