// SPDX-License-Identifier: AGPL-3.0-only
//! 書き出しの記録のファイルに書いた `id` のブルームフィルタ（ST33 / design D10 / D17）。
//!
//! 台帳の `written` に置く。記録の値も `id` そのものも取り出せない。偽陽性は 10⁻⁶ 程度で、
//! **入っていたものを落とさない**（誤りは多く数える側にしか出ない）。ハッシュは `sha2` で固定する
//! （標準のハッシャは版で変わりうるが、台帳に置いたフィルタは後の版のサーバが読む）。

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

    #[test]
    fn a_wrong_shape_is_not_read() {
        assert!(Bloom::from_bytes(&[]).is_none());
        assert!(Bloom::from_bytes(&[9, 20, 1]).is_none());
        assert!(Bloom::from_bytes(&[VERSION, 20]).is_none());
    }
}
