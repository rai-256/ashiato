// SPDX-License-Identifier: AGPL-3.0-only
//! 待ち受けと DB の接続先の検査（ST28 / design D7・D14）。
//! 拒否の理由は種別だけを返す。アドレスや接続先の値は出さない（製造準備 A-2 / C3）。

use std::net::{IpAddr, SocketAddr};
use std::str::FromStr as _;

use sqlx::postgres::PgConnectOptions;

/// 待ち受けを拒む理由の種別（design D7）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindRefusal {
    /// `0.0.0.0` / `::`（全インタフェース）。
    Unspecified,
    /// loopback 以外のアドレス。
    NotLoopback,
}

impl BindRefusal {
    /// ログに出す `kind`。
    pub fn kind(self) -> &'static str {
        match self {
            Self::Unspecified => "bind_unspecified",
            Self::NotLoopback => "bind_not_loopback",
        }
    }
}

/// 待ち受けてよいのは loopback（`127.0.0.0/8` / `::1`）だけ。
pub fn bind_allowed(addr: &SocketAddr) -> Result<(), BindRefusal> {
    let ip = addr.ip();
    if ip.is_unspecified() {
        Err(BindRefusal::Unspecified)
    } else if ip.is_loopback() {
        Ok(())
    } else {
        Err(BindRefusal::NotLoopback)
    }
}

/// `BIND` を解決したすべてのアドレスに `bind_allowed` を掛ける。解決できなければ Err。
pub async fn check_bind(bind: &str) -> anyhow::Result<Result<(), BindRefusal>> {
    let addrs: Vec<SocketAddr> = tokio::net::lookup_host(bind).await?.collect();
    anyhow::ensure!(!addrs.is_empty(), "BIND がアドレスに解決できない");
    Ok(addrs.iter().try_for_each(bind_allowed))
}

/// DB の接続先が loopback（`127.0.0.0/8` / `::1` / `localhost`）か unix socket か（design D14）。
/// 名前は解決しない（`localhost` 以外の名前は、解決先が変わりうるので通さない）。
/// URL が読めないときは false（接続しない側に倒す）。
pub fn db_host_is_local(url: &str) -> bool {
    let Ok(opts) = PgConnectOptions::from_str(url) else {
        return false;
    };
    let host = opts.get_host();
    if host.starts_with('/') || host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    host.trim_matches(['[', ']'])
        .parse::<IpAddr>()
        .is_ok_and(|ip| ip.is_loopback())
}

/// 拒否を出して終了コード 2 で終わる。値は出さず種別だけ。
pub fn refuse(kind: &'static str) -> ! {
    tracing::error!(kind = kind, "起動しない");
    eprintln!("error: kind={kind}");
    std::process::exit(2);
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn sa(s: &str) -> SocketAddr {
        s.parse().unwrap()
    }

    #[test]
    fn server_startup_bind_allowed_loopback() {
        assert_eq!(bind_allowed(&sa("127.0.0.1:1")), Ok(()));
        assert_eq!(bind_allowed(&sa("127.0.0.2:1")), Ok(()));
        assert_eq!(bind_allowed(&sa("[::1]:1")), Ok(()));
    }

    #[test]
    fn server_startup_bind_allowed_refuses_unspecified_and_others() {
        assert_eq!(
            bind_allowed(&sa("0.0.0.0:1")),
            Err(BindRefusal::Unspecified)
        );
        assert_eq!(bind_allowed(&sa("[::]:1")), Err(BindRefusal::Unspecified));
        assert_eq!(
            bind_allowed(&sa("192.0.2.1:1")),
            Err(BindRefusal::NotLoopback)
        );
    }

    #[tokio::test]
    async fn server_startup_bind_allowed_localhost_resolves_to_loopback_only() {
        assert_eq!(check_bind("localhost:1").await.unwrap(), Ok(()));
        assert_eq!(
            check_bind("192.0.2.1:1").await.unwrap(),
            Err(BindRefusal::NotLoopback)
        );
    }

    #[test]
    fn server_startup_db_not_loopback_host_rules() {
        for ok in [
            "postgres://u@127.0.0.1:5432/d",
            "postgres://u@127.0.0.9/d",
            "postgres://u@[::1]:5432/d",
            "postgres://u@localhost/d",
            "postgres://u@localhost/d?host=/var/run/postgresql",
        ] {
            assert!(db_host_is_local(ok), "{ok}");
        }
        for ng in [
            "postgres://u@192.0.2.1:5432/d",
            "postgres://u@0.0.0.0/d",
            "postgres://u@db.example.com/d",
            "not a url",
        ] {
            assert!(!db_host_is_local(ng), "{ng}");
        }
    }
}
