"""Generate the source-controlled app icon without external image dependencies."""
import pathlib
import struct
import zlib

size = 1024
pixels = bytearray()
for y in range(size):
    pixels.append(0)
    for x in range(size):
        sx, sy = x * 32 // size, y * 32 // size
        stroke = (8 <= sx < 24 and (7 <= sy < 11 or 14 <= sy < 18 or 21 <= sy < 25)
                  or 8 <= sx < 12 and 10 <= sy < 16 or 20 <= sx < 24 and 17 <= sy < 23)
        pixels.extend((129, 166, 239, 255) if stroke else (20, 22, 27, 255))


def chunk(kind, data):
    return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))


png = b'\x89PNG\r\n\x1a\n'
png += chunk(b'IHDR', struct.pack('>IIBBBBB', size, size, 8, 6, 0, 0, 0))
png += chunk(b'IDAT', zlib.compress(pixels, 9)) + chunk(b'IEND', b'')
(pathlib.Path(__file__).resolve().parent.parent / 'assets' / 'icon.png').write_bytes(png)
