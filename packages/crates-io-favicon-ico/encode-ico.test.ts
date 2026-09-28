import { describe, expect, test } from 'vitest';

import encodeIco from './encode-ico.js';

interface Frame {
  size: number;
  data: Buffer;
}

const HEADER_SIZE = 6;
const ENTRY_SIZE = 16;

function entry(ico: Buffer, index: number) {
  let base = HEADER_SIZE + ENTRY_SIZE * index;
  return {
    width: ico.readUInt8(base),
    height: ico.readUInt8(base + 1),
    planes: ico.readUInt16LE(base + 4),
    bpp: ico.readUInt16LE(base + 6),
    dataSize: ico.readUInt32LE(base + 8),
    offset: ico.readUInt32LE(base + 12),
  };
}

describe('encodeIco', () => {
  test('writes an ICO header with the frame count', () => {
    let ico = encodeIco([{ size: 16, data: Buffer.from([1, 2, 3]) }]);

    expect(ico.readUInt16LE(0)).toBe(0); // reserved
    expect(ico.readUInt16LE(2)).toBe(1); // type: icon
    expect(ico.readUInt16LE(4)).toBe(1); // frame count
  });

  test('describes each frame in the directory', () => {
    let frames: Frame[] = [
      { size: 16, data: Buffer.alloc(10, 0xaa) },
      { size: 32, data: Buffer.alloc(20, 0xbb) },
      { size: 48, data: Buffer.alloc(30, 0xcc) },
    ];
    let ico = encodeIco(frames);

    expect(ico.readUInt16LE(4)).toBe(3);

    let dirSize = HEADER_SIZE + ENTRY_SIZE * frames.length;
    expect(entry(ico, 0)).toEqual({ width: 16, height: 16, planes: 1, bpp: 32, dataSize: 10, offset: dirSize });
    expect(entry(ico, 1)).toEqual({ width: 32, height: 32, planes: 1, bpp: 32, dataSize: 20, offset: dirSize + 10 });
    expect(entry(ico, 2)).toEqual({ width: 48, height: 48, planes: 1, bpp: 32, dataSize: 30, offset: dirSize + 30 });
  });

  test('appends frame data at the offset each entry points to', () => {
    let frames: Frame[] = [
      { size: 16, data: Buffer.from([1, 1, 1]) },
      { size: 32, data: Buffer.from([2, 2]) },
    ];
    let ico = encodeIco(frames);

    for (let [i, frame] of frames.entries()) {
      let { offset, dataSize } = entry(ico, i);
      expect(ico.subarray(offset, offset + dataSize)).toEqual(frame.data);
    }
  });

  test('encodes a 256 pixel frame as a zero width/height byte', () => {
    let ico = encodeIco([{ size: 256, data: Buffer.from([0]) }]);

    expect(entry(ico, 0).width).toBe(0);
    expect(entry(ico, 0).height).toBe(0);
  });
});
