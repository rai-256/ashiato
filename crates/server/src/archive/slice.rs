// SPDX-License-Identifier: AGPL-3.0-only
//! JSON配列の各要素を再直列化せず、元のバイト列から切り出す。

/// 区切りをまたぐ状態を保ちながら、巨大な配列を固定幅で走査する。
const SCAN_CHUNK_BYTES: usize = 1024 * 1024;
const MAX_ITEM_BYTES: usize = 64 * 1024 * 1024;

/// JSON 配列を入力全体へ展開せず、完成した項目だけを順に渡す。
///
/// `receive` が返るまで次の項目を読み進めないので、呼び出し側が同時に持つ
/// 項目は常に 1 件である。各項目は原文のバイト列を保つが、64 MiB を超える
/// ものは壊れた項目として止める。
pub fn stream_array_items<R, F>(mut reader: R, mut receive: F) -> anyhow::Result<()>
where
    R: std::io::Read,
    F: FnMut(&[u8]) -> anyhow::Result<()>,
{
    let mut buffer = vec![0_u8; SCAN_CHUNK_BYTES];
    let mut item = Vec::new();
    let mut started = false;
    let mut depth = 0usize;
    let mut string = false;
    let mut escape = false;

    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        for byte in buffer[..read].iter().copied() {
            if started {
                item.push(byte);
                if item.len() > MAX_ITEM_BYTES {
                    anyhow::bail!("JSON 配列の項目が 64 MiB を超える");
                }
            }
            if string {
                if escape {
                    escape = false;
                } else if byte == b'\\' {
                    escape = true;
                } else if byte == b'"' {
                    string = false;
                }
                continue;
            }
            if byte == b'"' {
                string = true;
                continue;
            }
            if !started && byte == b'{' {
                started = true;
                depth = 1;
                item.push(byte);
                continue;
            }
            if started {
                if byte == b'{' || byte == b'[' {
                    depth += 1;
                }
                if byte == b'}' || byte == b']' {
                    depth -= 1;
                    if depth == 0 {
                        std::str::from_utf8(&item)
                            .map_err(|_| anyhow::anyhow!("UTF-8 ではない配列項目"))?;
                        receive(&item)?;
                        item.clear();
                        started = false;
                    }
                }
            }
        }
    }
    if started || string {
        anyhow::bail!("壊れたJSON配列")
    }
    Ok(())
}

pub fn array_items(input: &[u8]) -> anyhow::Result<Vec<&[u8]>> {
    let mut out = Vec::new();
    let mut start = None;
    let mut depth = 0usize;
    let mut string = false;
    let mut escape = false;
    for (chunk_index, chunk) in input.chunks(SCAN_CHUNK_BYTES).enumerate() {
        for (offset, byte) in chunk.iter().copied().enumerate() {
            let i = chunk_index * SCAN_CHUNK_BYTES + offset;
            if string {
                if escape {
                    escape = false;
                } else if byte == b'\\' {
                    escape = true;
                } else if byte == b'"' {
                    string = false;
                }
                continue;
            }
            if byte == b'"' {
                string = true;
                continue;
            }
            if start.is_none() && byte == b'{' {
                start = Some(i);
                depth = 1;
                continue;
            }
            if start.is_some() {
                if byte == b'{' || byte == b'[' {
                    depth += 1;
                }
                if byte == b'}' || byte == b']' {
                    depth -= 1;
                    if depth == 0 {
                        if let Some(begin) = start.take() {
                            let item = &input[begin..=i];
                            std::str::from_utf8(item)
                                .map_err(|_| anyhow::anyhow!("UTF-8 ではない配列項目"))?;
                            out.push(item);
                        }
                    }
                }
            }
        }
    }
    if start.is_some() || string {
        anyhow::bail!("壊れたJSON配列")
    }
    Ok(out)
}
