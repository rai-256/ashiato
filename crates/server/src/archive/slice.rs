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

/// 空白を飛ばした先の位置。
fn skip_ws(input: &[u8], mut i: usize) -> usize {
    while i < input.len() && matches!(input[i], b' ' | b'\t' | b'\n' | b'\r') {
        i += 1;
    }
    i
}

/// `start`（値の先頭）から始まる JSON の値 1 つの終わり（その次の位置）を返す。
///
/// 中を解釈しない —— 文字列の中か・入れ子の深さだけを持って境目を探す。
fn value_end(input: &[u8], start: usize) -> anyhow::Result<usize> {
    let first = *input
        .get(start)
        .ok_or_else(|| anyhow::anyhow!("JSON の値が途中で切れている"))?;
    match first {
        b'"' => {
            let mut escape = false;
            for (offset, byte) in input[start + 1..].iter().copied().enumerate() {
                if escape {
                    escape = false;
                } else if byte == b'\\' {
                    escape = true;
                } else if byte == b'"' {
                    return Ok(start + 1 + offset + 1);
                }
            }
            anyhow::bail!("JSON の文字列が閉じていない")
        }
        b'{' | b'[' => {
            let mut depth = 0usize;
            let mut string = false;
            let mut escape = false;
            for (offset, byte) in input[start..].iter().copied().enumerate() {
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
                match byte {
                    b'"' => string = true,
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => {
                        depth -= 1;
                        if depth == 0 {
                            return Ok(start + offset + 1);
                        }
                    }
                    _ => {}
                }
            }
            anyhow::bail!("JSON の入れ子が閉じていない")
        }
        _ => {
            let mut end = start;
            while end < input.len()
                && !matches!(
                    input[end],
                    b',' | b']' | b'}' | b' ' | b'\t' | b'\n' | b'\r'
                )
            {
                end += 1;
            }
            if end == start {
                anyhow::bail!("JSON の値が無い");
            }
            Ok(end)
        }
    }
}

/// オブジェクト（`input` 全体）の欄 `key` の値の**元のバイト列**。無ければ `None`。
///
/// 書庫の原文を切り出すため（spec「原文は書庫のバイト列の一部と一致する」）。
/// 配列の中の項目だけでなく、`{"semanticSegments":[…]}` の中の項目や、
/// セグメントの中の `timelinePath` の点も、ここを重ねて辿る（final review R55）。
pub fn object_member<'a>(input: &'a [u8], key: &str) -> anyhow::Result<Option<&'a [u8]>> {
    let mut i = skip_ws(input, 0);
    if input.get(i) != Some(&b'{') {
        anyhow::bail!("JSON のオブジェクトでない");
    }
    i = skip_ws(input, i + 1);
    if input.get(i) == Some(&b'}') {
        return Ok(None);
    }
    loop {
        if input.get(i) != Some(&b'"') {
            anyhow::bail!("JSON の欄名が無い");
        }
        let key_end = value_end(input, i)?;
        let name: String = serde_json::from_slice(&input[i..key_end])?;
        i = skip_ws(input, key_end);
        if input.get(i) != Some(&b':') {
            anyhow::bail!("JSON の欄名の後に : が無い");
        }
        let value_start = skip_ws(input, i + 1);
        let value_stop = value_end(input, value_start)?;
        if name == key {
            return Ok(Some(&input[value_start..value_stop]));
        }
        i = skip_ws(input, value_stop);
        match input.get(i) {
            Some(b',') => i = skip_ws(input, i + 1),
            Some(b'}') => return Ok(None),
            _ => anyhow::bail!("JSON のオブジェクトが壊れている"),
        }
    }
}

/// 配列（`input` 全体）の各要素の**元のバイト列**。`null` や数値の要素も 1 つと数えるので、
/// 解釈した配列と添字が必ず揃う（`array_items` は `{…}` しか拾わず、ずれうる。review I3）。
pub fn array_elements(input: &[u8]) -> anyhow::Result<Vec<&[u8]>> {
    let mut i = skip_ws(input, 0);
    if input.get(i) != Some(&b'[') {
        anyhow::bail!("JSON の配列でない");
    }
    let mut out = Vec::new();
    i = skip_ws(input, i + 1);
    if input.get(i) == Some(&b']') {
        return Ok(out);
    }
    loop {
        let stop = value_end(input, i)?;
        let item = &input[i..stop];
        std::str::from_utf8(item).map_err(|_| anyhow::anyhow!("UTF-8 ではない配列項目"))?;
        out.push(item);
        i = skip_ws(input, stop);
        match input.get(i) {
            Some(b',') => i = skip_ws(input, i + 1),
            Some(b']') => return Ok(out),
            _ => anyhow::bail!("JSON の配列が壊れている"),
        }
    }
}
