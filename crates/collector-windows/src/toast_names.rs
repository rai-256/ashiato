//! 通知の差出人名。Windows API に依存しない系統ごとの値。

/// Scenario: 開発版の PC の収集アプリは、開発版の自動起動の項目と差出人を使う
#[cfg(test)]
mod tests {
    #[test]
    fn st54_dev_toast_sender_uses_dev_values() {
        assert_eq!(
            super::aumid(ashiato_build_stamp::Channel::Dev),
            "dev.ashiato.collector.dev"
        );
        assert_eq!(
            super::display_name(ashiato_build_stamp::Channel::Dev),
            "ashiato 開発版"
        );
    }
}

pub const fn aumid(channel: ashiato_build_stamp::Channel) -> &'static str {
    match channel {
        ashiato_build_stamp::Channel::Dev => "dev.ashiato.collector.dev",
        ashiato_build_stamp::Channel::Official => "dev.ashiato.collector",
    }
}

pub const fn display_name(channel: ashiato_build_stamp::Channel) -> &'static str {
    match channel {
        ashiato_build_stamp::Channel::Dev => "ashiato 開発版",
        ashiato_build_stamp::Channel::Official => "ashiato",
    }
}
