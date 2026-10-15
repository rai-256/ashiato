// SPDX-License-Identifier: AGPL-3.0-only
//! 書き出しの記録のファイルに書いた `id` のブルームフィルタ（ST33 / design D10 / D17）。
//!
//! 台帳の `written` に置く。記録の値も `id` そのものも取り出せない。偽陽性は 10⁻⁶ 程度で、
//! **入っていたものを落とさない**（誤りは多く数える側にしか出ない）。ハッシュは `sha2` で固定する
//! （標準のハッシャは版で変わりうるが、台帳に置いたフィルタは後の版のサーバが読む）。
//!
//! **形は凍結する**（deep Q12 / R20）。台帳は追記のみで `written` を書き直せないので、位置の取り方・形を変えると
//! 前に書いた行を読めなくなる。試験 `the_frozen_shape_is_kept` が別の実装で作ったバイト列を期待値に持つ。
//! 読めない形の行は 0 件ではなく「数えられない」になる（`ledger::erased_after`）。

use sha2::{Digest as _, Sha256};
use uuid::Uuid;

/// 形式の版（先頭の 1 バイト）。
const VERSION: u8 = 1;
/// 偽陽性 10⁻⁶ のときの 1 要素あたりのビット数（-ln p / (ln 2)²）と、ハッシュの数。
const BITS_PER_ITEM: f64 = 28.76;
const HASHES: u8 = 20;

/// 書き込み側と読み取り側で同じ形。`[版][ハッシュの数][ビット列…]`。
#[derive(Debug)]
pub struct Bloom {
    hashes: u8,
    bits: Vec<u8>,
}

impl Bloom {
    /// `expected` 個を入れる大きさで作る。
    pub fn with_capacity(expected: u64) -> Self {
        let bits = ((expected.max(1) as f64 * BITS_PER_ITEM) / 8.0).ceil() as usize;
        Self {
            hashes: HASHES,
            bits: vec![0; bits.max(8)],
        }
    }

    fn positions(&self, id: Uuid) -> impl Iterator<Item = usize> {
        let digest = Sha256::digest(id.as_bytes());
        let h1 = u64::from_le_bytes(digest[0..8].try_into().unwrap_or_default());
        let h2 = u64::from_le_bytes(digest[8..16].try_into().unwrap_or_default()) | 1;
        let m = (self.bits.len() * 8) as u64;
        (0..u64::from(self.hashes)).map(move |i| (h1.wrapping_add(i.wrapping_mul(h2)) % m) as usize)
    }

    pub fn insert(&mut self, id: Uuid) {
        for p in self.positions(id).collect::<Vec<_>>() {
            self.bits[p / 8] |= 1 << (p % 8);
        }
    }

    pub fn contains(&self, id: Uuid) -> bool {
        self.positions(id)
            .all(|p| self.bits[p / 8] & (1 << (p % 8)) != 0)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = vec![VERSION, self.hashes];
        out.extend_from_slice(&self.bits);
        out
    }

    /// 台帳から読む。形が違えば `None`。
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        match bytes {
            [VERSION, hashes, bits @ ..] if *hashes > 0 && !bits.is_empty() => Some(Self {
                hashes: *hashes,
                bits: bits.to_vec(),
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn never_drops_an_inserted_id_and_survives_a_round_trip() {
        let ids: Vec<Uuid> = (0..2000).map(|_| Uuid::new_v4()).collect();
        let mut b = Bloom::with_capacity(ids.len() as u64);
        for id in &ids {
            b.insert(*id);
        }
        let back = Bloom::from_bytes(&b.to_bytes()).unwrap();
        assert!(ids.iter().all(|id| back.contains(*id)));
        let false_hits = (0..20_000)
            .filter(|_| back.contains(Uuid::new_v4()))
            .count();
        assert!(false_hits <= 2, "偽陽性が多い: {false_hits}");
    }

    /// 形の凍結（deep Q12 / R20）。期待値は `bloom.rs` を使わずに作った（Python の `hashlib` で同じ形を書いた別の実装）。
    /// 位置の取り方・ハッシュの数・大きさの決め方・先頭の 2 バイトのどれを変えても落ちる。変えるなら `VERSION` を上げ、
    /// 前の版の行を読む口を残す（読めない行は「数えられない」になる）。
    #[test]
    fn the_frozen_shape_is_kept() {
        let ids = [
            "00000000-0000-0000-0000-000000000001",
            "6f1c2d3e-4a5b-4c6d-8e7f-90a1b2c3d4e5",
            "ffffffff-ffff-ffff-ffff-ffffffffffff",
        ]
        .map(|s| Uuid::parse_str(s).unwrap());
        let mut b = Bloom::with_capacity(ids.len() as u64);
        for id in ids {
            b.insert(id);
        }
        let frozen: [u8; 13] = [
            0x01, 0x14, 0x4c, 0x90, 0x53, 0x4d, 0x27, 0x95, 0x7a, 0x8c, 0xa9, 0xf6, 0x2a,
        ];
        assert_eq!(b.to_bytes(), frozen);
        let back = Bloom::from_bytes(&frozen).unwrap();
        assert!(ids.iter().all(|id| back.contains(*id)));
    }

    #[test]
    fn a_wrong_shape_is_not_read() {
        assert!(Bloom::from_bytes(&[]).is_none());
        assert!(Bloom::from_bytes(&[9, 20, 1]).is_none());
        assert!(Bloom::from_bytes(&[VERSION, 20]).is_none());
    }
}
