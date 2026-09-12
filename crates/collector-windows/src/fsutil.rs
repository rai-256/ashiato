// SPDX-License-Identifier: AGPL-3.0-only
//! 置き場への書き込み。**書きかけで落ちても、前の中身か新しい中身のどちらかが残る。**
//!
//! `std::fs::write` はファイルを切り詰めてから書くので、その間に電源が落ちると
//! **空のファイルが残る**。印（`last-seen.txt`）と数え（`counters.json`）は
//! 空を「壊れている」として読むので、1 回の書きかけで以後の起動がすべて止まった
//! （review/code.md R18）。
use anyhow::Context as _;

/// 一時ファイルへ書いて同期してから置き換える。
pub fn atomic_write(path: &std::path::Path, bytes: &[u8]) -> anyhow::Result<()> {
    use std::io::Write as _;

    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).context("置き場を作れない")?;
    }
    let tmp = path.with_extension("tmp");
    {
        let mut f = std::fs::File::create(&tmp).context("一時ファイルを作れない")?;
        f.write_all(bytes).context("一時ファイルへ書けない")?;
        // **置き換える前に同期する** —— 名前だけが先に見えて中身が空、を防ぐ
        f.sync_all().context("一時ファイルを同期できない")?;
    }
    std::fs::rename(&tmp, path).context("置き換えられない")
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;

    /// 置き換えた後は新しい中身だけが残り、一時ファイルは残らない。
    #[test]
    fn atomic_write_replaces_and_leaves_no_tmp() {
        let dir = std::env::temp_dir().join(format!("ashiato-fs-{}", uuid::Uuid::new_v4()));
        let path = dir.join("a.txt");
        atomic_write(&path, b"one").unwrap();
        atomic_write(&path, b"two").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "two");
        assert!(!path.with_extension("tmp").exists());
        std::fs::remove_dir_all(&dir).ok();
    }
}
