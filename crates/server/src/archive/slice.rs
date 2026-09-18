// SPDX-License-Identifier: AGPL-3.0-only
//! JSON配列の各要素を再直列化せず、元のバイト列から切り出す。

pub fn array_items(input: &[u8]) -> anyhow::Result<Vec<&[u8]>> {
    let mut out = Vec::new();
    let mut start = None;
    let mut depth = 0usize;
    let mut string = false;
    let mut escape = false;
    for (i, byte) in input.iter().copied().enumerate() {
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
                        out.push(&input[begin..=i]);
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
