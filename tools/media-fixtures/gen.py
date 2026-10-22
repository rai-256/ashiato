#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only
"""ST09 の試験材料（JPEG 2 枚と MP4 1 本）を作る。外の写真・動画は使わない（自作。ライセンスの問題が無い）。

  java MkBase.java /tmp/base.jpg 64 48 && java MkBase.java /tmp/thumb.jpg 16 12
  python3 gen.py /tmp/base.jpg /tmp/thumb.jpg ../../collector-android/app/src/test/resources/media

標準ライブラリだけ。EXIF（TIFF）は手で組む —— 欄の値は試験が名指しで固定する（README.md 参照）。
"""
import datetime
import struct
import sys

BE = ">"


def entry(tag, typ, count, value_or_off):
    return struct.pack(BE + "HHI", tag, typ, count) + value_or_off


class Ifd:
    """1 つの IFD。欄を足し、置き場所（TIFF 先頭からの位置）が決まってから bytes にする。"""

    def __init__(self):
        self.items = []  # (tag, type, count, payload bytes)

    def add(self, tag, typ, count, payload):
        self.items.append((tag, typ, count, payload))

    def ascii(self, tag, s):
        b = s.encode("ascii") + b"\0"
        self.add(tag, 2, len(b), b)

    def short(self, tag, v):
        self.add(tag, 3, 1, struct.pack(BE + "H", v))

    def long(self, tag, v):
        self.add(tag, 4, 1, struct.pack(BE + "I", v))

    def rational(self, tag, pairs):
        self.add(tag, 5, len(pairs), b"".join(struct.pack(BE + "II", n, d) for n, d in pairs))

    def undefined(self, tag, b):
        self.add(tag, 7, len(b), b)

    def size(self):
        out = sum(len(p) + len(p) % 2 for _, _, _, p in self.items if len(p) > 4)
        return 2 + 12 * len(self.items) + 4 + out

    def build(self, at, next_ifd=0, patch=None):
        """patch: {tag: 値}（他の IFD / 縮小画像への位置。置き場所が決まった後で入れる）"""
        items = sorted(self.items, key=lambda i: i[0])
        head = struct.pack(BE + "H", len(items))
        body = b""
        data_at = at + 2 + 12 * len(items) + 4
        data = b""
        for tag, typ, count, payload in items:
            if patch and tag in patch:
                payload = struct.pack(BE + "I", patch[tag])
            if len(payload) <= 4:
                body += entry(tag, typ, count, payload.ljust(4, b"\0"))
            else:
                body += entry(tag, typ, count, struct.pack(BE + "I", data_at + len(data)))
                data += payload + (b"\0" if len(payload) % 2 else b"")
        return head + body + struct.pack(BE + "I", next_ifd) + data


def app1(with_gps, with_thumb, thumb, offset):
    ifd0, exif, gps, ifd1 = Ifd(), Ifd(), Ifd(), Ifd()
    ifd0.ascii(0x010F, "AshiatoTest")  # Make
    ifd0.ascii(0x0110, "Fixture One")  # Model
    ifd0.short(0x0112, 1)  # Orientation
    ifd0.ascii(0x010E, "ashiato2 ST09 fixture")  # ImageDescription
    ifd0.ascii(0x0132, "2026:10:17 14:30:15")  # DateTime
    ifd0.long(0x8769, 0)  # ExifIFD pointer（後で入れる）
    if with_gps:
        ifd0.long(0x8825, 0)  # GPS pointer
    exif.rational(0x829A, [(1, 125)])  # ExposureTime
    exif.rational(0x829D, [(18, 10)])  # FNumber
    exif.short(0x8827, 200)  # ISOSpeedRatings
    exif.undefined(0x9000, b"0232")  # ExifVersion
    exif.ascii(0x9003, "2026:10:17 14:30:12")  # DateTimeOriginal
    exif.ascii(0x9004, "2026:10:17 14:30:12")  # DateTimeDigitized
    if offset:
        exif.ascii(0x9011, offset)  # OffsetTimeOriginal
    # MakerNote（機種固有のバイナリ。300 バイト = 256 を超える）
    exif.undefined(0x927C, bytes((i * 7 + 3) % 251 for i in range(300)))
    if with_gps:
        gps.add(0x0000, 1, 4, bytes([2, 3, 0, 0]))  # GPSVersionID
        gps.ascii(0x0001, "N")
        gps.rational(0x0002, [(35, 1), (40, 1), (5232, 100)])  # 35.6812
        gps.ascii(0x0003, "E")
        gps.rational(0x0004, [(139, 1), (46, 1), (156, 100)])  # 139.7671
    if with_thumb:
        ifd1.short(0x0103, 6)  # Compression = JPEG
        ifd1.long(0x0201, 0)  # JPEGInterchangeFormat
        ifd1.long(0x0202, len(thumb))  # JPEGInterchangeFormatLength
    pos = 8
    p0 = pos
    pos += ifd0.size()
    pe = pos
    pos += exif.size()
    pg = pos
    if with_gps:
        pos += gps.size()
    p1 = pos
    if with_thumb:
        pos += ifd1.size()
    pt = pos
    patch0 = {0x8769: pe}
    if with_gps:
        patch0[0x8825] = pg
    tiff = struct.pack(BE + "2sHI", b"MM", 42, 8)
    tiff += ifd0.build(p0, next_ifd=p1 if with_thumb else 0, patch=patch0)
    tiff += exif.build(pe)
    if with_gps:
        tiff += gps.build(pg)
    if with_thumb:
        tiff += ifd1.build(p1, patch={0x0201: pt})
        tiff += thumb
    seg = b"Exif\0\0" + tiff
    return b"\xff\xe1" + struct.pack(">H", len(seg) + 2) + seg


def jpeg(base, **kw):
    assert base[:2] == b"\xff\xd8"
    return base[:2] + app1(**kw) + base[2:]  # SOI の直後に APP1


def box(typ, payload=b""):
    return struct.pack(">I4s", 8 + len(payload), typ) + payload


def mp4():
    """位置（©xyz）と長さ 12.5 秒を持つ最小の MP4。映像の trak に標本を 1 つ持つ（中身は復号できない。メタだけを読む試験の材料）。"""
    # 2026-10-17T05:30:12Z（1904 年起点の秒）
    created = int((datetime.datetime(2026, 10, 17, 5, 30, 12, tzinfo=datetime.timezone.utc)
                   - datetime.datetime(1904, 1, 1, tzinfo=datetime.timezone.utc)).total_seconds())
    matrix = struct.pack(">9I", 0x10000, 0, 0, 0, 0x10000, 0, 0, 0, 0x40000000)
    mvhd = box(b"mvhd", struct.pack(">I", 0) + struct.pack(">IIII", created, created, 1000, 12500)
               + struct.pack(">IH", 0x10000, 0x100) + b"\0" * 10 + matrix + b"\0" * 24 + struct.pack(">I", 2))
    tkhd = box(b"tkhd", struct.pack(">I", 3) + struct.pack(">IIIII", created, created, 1, 0, 12500)
               + b"\0" * 8 + struct.pack(">HHHH", 0, 0, 0, 0) + matrix + struct.pack(">II", 64 << 16, 48 << 16))
    mdhd = box(b"mdhd", struct.pack(">I", 0) + struct.pack(">IIII", created, created, 1000, 12500)
               + struct.pack(">HH", 0x55C4, 0))
    hdlr = box(b"hdlr", struct.pack(">I", 0) + b"\0\0\0\0vide" + b"\0" * 12 + b"VideoHandler\0")
    avc1 = box(b"avc1", b"\0" * 6 + struct.pack(">H", 1) + b"\0" * 16 + struct.pack(">HH", 64, 48)
               + struct.pack(">IIIH", 0x480000, 0x480000, 0, 1) + b"\0" * 32 + struct.pack(">Hh", 24, -1)
               + box(b"avcC", bytes([1, 0x42, 0xC0, 0x1E, 0xFF, 0xE1, 0, 4, 0x67, 0x42, 0xC0, 0x1E, 1, 0, 4, 0x68, 0xCE, 0x06, 0xE2])))
    stsd = box(b"stsd", struct.pack(">II", 0, 1) + avc1)
    stts = box(b"stts", struct.pack(">IIII", 0, 1, 1, 12500))
    stsc = box(b"stsc", struct.pack(">IIIII", 0, 1, 1, 1, 1))
    stsz = box(b"stsz", struct.pack(">IIII", 0, 0, 1, 8))
    stbl_pre = stsd + stts + stsc + stsz
    dinf = box(b"dinf", box(b"dref", struct.pack(">II", 0, 1) + box(b"url ", struct.pack(">I", 1))))
    vmhd = box(b"vmhd", struct.pack(">IHHHH", 1, 0, 0, 0, 0))
    loc = b"+35.6812+139.7671/"
    udta = box(b"udta", box(b"\xa9xyz", struct.pack(">HH", len(loc), 0x15C7) + loc))
    ftyp = box(b"ftyp", b"isom" + struct.pack(">I", 512) + b"isomiso2avc1mp41")

    def moov(stco_off):
        stco = box(b"stco", struct.pack(">III", 0, 1, stco_off))
        minf = box(b"minf", vmhd + dinf + box(b"stbl", stbl_pre + stco))
        trak = box(b"trak", tkhd + box(b"mdia", mdhd + hdlr + minf))
        return box(b"moov", mvhd + trak + udta)

    mdat_off = len(ftyp) + len(moov(0)) + 8
    return ftyp + moov(mdat_off) + box(b"mdat", b"ASHIATO2")


def main():
    base = open(sys.argv[1], "rb").read()
    thumb = open(sys.argv[2], "rb").read()
    out = sys.argv[3]
    open(out + "/camera-gps.jpg", "wb").write(
        jpeg(base, with_gps=True, with_thumb=True, thumb=thumb, offset="+09:00"))
    open(out + "/camera-no-offset.jpg", "wb").write(
        jpeg(base, with_gps=False, with_thumb=False, thumb=b"", offset=None))
    open(out + "/video-gps.mp4", "wb").write(mp4())


if __name__ == "__main__":
    main()
