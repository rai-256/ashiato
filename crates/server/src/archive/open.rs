// SPDX-License-Identifier: AGPL-3.0-only
//! 書庫を開く境界。解析器はここが返す中身だけを扱う（ST12 / D3）。

use std::io::Read as _;
use std::path::Path;

/// zip の中の 1 ファイル。パスは書庫内の相対パスだけを持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveFile {
    pub path: String,
    pub bytes: Vec<u8>,
}

/// 読めなかった書庫を台帳へ残すための、値を含まない失敗種別。
#[derive(Debug)]
pub struct OpenError {
    kind: &'static str,
    source: Option<zip::result::ZipError>,
}

impl OpenError {
    pub fn kind(&self) -> &'static str {
        self.kind
    }
}

impl std::fmt::Display for OpenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)
    }
}

impl std::error::Error for OpenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|error| error as _)
    }
}

/// zip を 1 本だけ開く。分割書庫は結合せず呼び出し側が 1 本ずつ渡す。
///
/// **裸の `.json` も 1 ファイルの書庫として開く** —— 端末から書き出した
/// `Timeline.json` は zip に包まれずに専用のフォルダへ置かれる（spec「専用の
/// フォルダでは、`.zip` の書庫と `.json` のファイルを読む」）。包んだものしか
/// 開けなかったときは、本人の決定 Q8 で唯一残した経路が塞がっていた（R5）。
pub fn open_archive(path: &Path) -> Result<Vec<ArchiveFile>, OpenError> {
    let extension = path.extension().and_then(|extension| extension.to_str());
    if extension == Some("json") {
        let bytes = std::fs::read(path).map_err(|_| OpenError {
            kind: "unsupported_format",
            source: None,
        })?;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("archive.json")
            .to_owned();
        return Ok(vec![ArchiveFile { path: name, bytes }]);
    }
    if extension != Some("zip") {
        return Err(OpenError {
            kind: "unsupported_format",
            source: None,
        });
    }
    let file = std::fs::File::open(path).map_err(|_| OpenError {
        kind: "broken_zip",
        source: None,
    })?;
    let mut archive = zip::ZipArchive::new(file).map_err(|source| OpenError {
        kind: "broken_zip",
        source: Some(source),
    })?;
    let mut files = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|source| OpenError {
            kind: "broken_zip",
            source: Some(source),
        })?;
        if entry.is_dir() {
            continue;
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).map_err(|_| OpenError {
            kind: "broken_zip",
            source: None,
        })?;
        files.push(ArchiveFile {
            path: entry.name().to_owned(),
            bytes,
        });
    }
    Ok(files)
}
