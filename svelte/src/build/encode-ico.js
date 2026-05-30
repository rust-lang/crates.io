const HEADER_SIZE = 6;
const ENTRY_SIZE = 16;

/**
 * Packs pre-rendered PNG buffers into an ICO container using PNG-compressed
 * frames. Every modern browser reads PNG-in-ICO frames, which are far smaller
 * than the uncompressed BMP frames that most ICO encoders emit.
 *
 * @param {Array<{ size: number, data: Buffer }>} frames square PNG frames,
 *   `size` being the edge length in pixels
 * @returns {Buffer}
 */
export default function encodeIco(frames) {
  let header = Buffer.alloc(HEADER_SIZE);
  header.writeUInt16LE(0, 0); // reserved, always 0
  header.writeUInt16LE(1, 2); // image type, 1 for icon
  header.writeUInt16LE(frames.length, 4);

  let directory = [];
  let dataOffset = HEADER_SIZE + ENTRY_SIZE * frames.length;
  for (let { size, data } of frames) {
    let entry = Buffer.alloc(ENTRY_SIZE);
    // 256 pixels does not fit a single byte and is encoded as 0.
    entry.writeUInt8(size >= 256 ? 0 : size, 0); // width
    entry.writeUInt8(size >= 256 ? 0 : size, 1); // height
    entry.writeUInt8(0, 2); // palette color count, 0 for a non-paletted frame
    entry.writeUInt8(0, 3); // reserved
    entry.writeUInt16LE(1, 4); // color planes
    entry.writeUInt16LE(32, 6); // bits per pixel
    entry.writeUInt32LE(data.length, 8);
    entry.writeUInt32LE(dataOffset, 12);
    directory.push(entry);
    dataOffset += data.length;
  }

  return Buffer.concat([header, ...directory, ...frames.map(frame => frame.data)]);
}
