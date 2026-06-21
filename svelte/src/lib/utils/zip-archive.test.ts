import type { ManifestFile } from './zip-archive';

import { db, handlers } from '@crates-io/msw';
import { http, HttpResponse } from 'msw';
import { setupServer } from 'msw/node';
import { afterAll, afterEach, beforeAll, describe, expect, it } from 'vitest';

import { archiveUrl, loadFile, loadManifest, manifestUrl } from './zip-archive';

const BASE = 'https://static.crates.io';
const CRATE = 'serde';
const VERSION = '1.0.228';
const ARCHIVE = `${BASE}/crates/${CRATE}/${CRATE}-${VERSION}.zip`;
const MANIFEST = `${ARCHIVE}.json`;

let server = setupServer(...handlers);
beforeAll(() => server.listen({ onUnhandledRequest: 'error' }));
afterEach(() => server.resetHandlers());
afterEach(() => db.reset());
afterAll(() => server.close());

const ENCODER = new TextEncoder();

function encode(text: string): Uint8Array {
  return ENCODER.encode(text);
}

/** Compresses bytes as a raw DEFLATE stream, mirroring how zip entries are stored. */
async function deflateRaw(data: Uint8Array): Promise<Uint8Array> {
  let stream = new Blob([data as BlobPart]).stream().pipeThrough(new CompressionStream('deflate-raw'));
  return new Uint8Array(await new Response(stream).arrayBuffer());
}

/** Returns the lowercase hex SHA-256 of the given bytes, matching the manifest format. */
async function sha256Hex(bytes: Uint8Array): Promise<string> {
  let digest = await crypto.subtle.digest('SHA-256', bytes as BufferSource);
  return [...new Uint8Array(digest)].map(byte => byte.toString(16).padStart(2, '0')).join('');
}

function manifestFile(overrides: Partial<ManifestFile> = {}): ManifestFile {
  return {
    path: 'src/lib.rs',
    data_offset: 0,
    compressed_size: 0,
    uncompressed_size: 0,
    compression: 'deflate',
    sha256: '',
    ...overrides,
  };
}

/** Publishes a crate version whose source archive the CDN mock will serve. */
async function publishSource(sourceFiles: Record<string, string>) {
  let crate = await db.crate.create({ name: CRATE });
  await db.version.create({ crate, num: VERSION, source_files: sourceFiles });
}

/** Loads a single file through the CDN mock, resolving its manifest entry first. */
async function loadServedFile(path: string) {
  let manifest = await loadManifest(fetch, BASE, CRATE, VERSION);
  let entry = manifest?.files.find(file => file.path === path);
  if (!entry) throw new Error(`manifest is missing "${path}"`);
  return loadFile(fetch, BASE, CRATE, VERSION, entry);
}

describe('archiveUrl()', () => {
  it('builds the archive URL from the CDN base', () => {
    let url = archiveUrl('https://static.crates.io', 'serde', '1.0.228');
    expect(url).toBe('https://static.crates.io/crates/serde/serde-1.0.228.zip');
  });

  it('produces a same-origin relative URL for an empty base', () => {
    let url = archiveUrl('', 'serde', '1.0.228');
    expect(url).toBe('/crates/serde/serde-1.0.228.zip');
  });
});

describe('manifestUrl()', () => {
  it('appends `.json` to the archive URL', () => {
    let url = manifestUrl('https://static.crates.io', 'serde', '1.0.228');
    expect(url).toBe('https://static.crates.io/crates/serde/serde-1.0.228.zip.json');
  });
});

describe('loadManifest()', () => {
  it('returns the manifest served by the CDN', async () => {
    await publishSource({ 'src/lib.rs': 'fn main() {}\n', 'Cargo.toml': 'name = "serde"\n' });

    let manifest = await loadManifest(fetch, BASE, CRATE, VERSION);
    expect(manifest).toMatchInlineSnapshot(`
      {
        "files": [
          {
            "compressed_size": 17,
            "compression": "deflate",
            "data_offset": 0,
            "path": "Cargo.toml",
            "sha256": "be3701dbfa0e482cdc1e8679e59cc8d9df3655562f4d8c4edba7bf9d906b1846",
            "uncompressed_size": 15,
          },
          {
            "compressed_size": 15,
            "compression": "deflate",
            "data_offset": 17,
            "path": "src/lib.rs",
            "sha256": "536e506bb90914c243a12b397b9a998f85ae2cbd9ba02dfd03a9e155ca5ca0f4",
            "uncompressed_size": 13,
          },
        ],
      }
    `);
  });

  it('returns null when the CDN returns 404', async () => {
    server.use(http.get(MANIFEST, () => new HttpResponse(null, { status: 404 })));
    expect(await loadManifest(fetch, BASE, CRATE, VERSION)).toBeNull();
  });

  it('returns null when the CDN returns 403', async () => {
    let crate = await db.crate.create({ name: CRATE });
    await db.version.create({ crate, num: VERSION });

    expect(await loadManifest(fetch, BASE, CRATE, VERSION)).toBeNull();
  });

  it('throws on any other error status', async () => {
    server.use(http.get(MANIFEST, () => new HttpResponse(null, { status: 500 })));
    await expect(loadManifest(fetch, BASE, CRATE, VERSION)).rejects.toThrowErrorMatchingInlineSnapshot(
      `[Error: Failed to load archive manifest (500 Internal Server Error)]`,
    );
  });
});

describe('loadFile()', () => {
  it('loads and inflates a file served by the CDN', async () => {
    let contents = 'fn main() { println!("hi"); }\n';
    await publishSource({ 'src/lib.rs': contents });

    expect(await loadServedFile('src/lib.rs')).toEqual({ kind: 'text', text: contents });
  });

  it('reads a stored (uncompressed) entry from a 206 range response', async () => {
    let text = 'name = "serde"\n';
    let stored = encode(text);
    let file = manifestFile({
      compression: 'store',
      compressed_size: stored.length,
      uncompressed_size: stored.length,
      sha256: await sha256Hex(stored),
    });

    server.use(http.get(ARCHIVE, () => new HttpResponse(stored, { status: 206 })));

    expect(await loadFile(fetch, BASE, CRATE, VERSION, file)).toEqual({ kind: 'text', text });
  });

  it('slices the entry out of a full 200 response', async () => {
    let text = 'pub fn answer() -> u32 { 42 }\n';
    let raw = encode(text);
    let compressed = await deflateRaw(raw);
    let offset = 64;

    // A range request can come back `200` with the whole archive. The entry's
    // compressed bytes sit at `data_offset`, surrounded by other entries.
    let archive = new Uint8Array(offset + compressed.length + 32);
    archive.fill(0xff);
    archive.set(compressed, offset);

    let file = manifestFile({
      data_offset: offset,
      compression: 'deflate',
      compressed_size: compressed.length,
      uncompressed_size: raw.length,
      sha256: await sha256Hex(raw),
    });

    server.use(http.get(ARCHIVE, () => new HttpResponse(archive, { status: 200 })));

    expect(await loadFile(fetch, BASE, CRATE, VERSION, file)).toEqual({ kind: 'text', text });
  });

  it('throws when the decompressed size differs from the manifest', async () => {
    let stored = encode('name = "serde"\n');
    let file = manifestFile({
      compression: 'store',
      compressed_size: stored.length,
      uncompressed_size: stored.length + 5,
      sha256: await sha256Hex(stored),
    });

    server.use(http.get(ARCHIVE, () => new HttpResponse(stored, { status: 206 })));

    await expect(loadFile(fetch, BASE, CRATE, VERSION, file)).rejects.toMatchInlineSnapshot(
      `[Error: Integrity check failed for "src/lib.rs": expected 20 bytes, got 15]`,
    );
  });

  it('throws when the SHA-256 differs from the manifest', async () => {
    let stored = encode('name = "serde"\n');
    let file = manifestFile({
      compression: 'store',
      compressed_size: stored.length,
      uncompressed_size: stored.length,
      sha256: 'da39a3ee5e6b4b0d3255bfef95601890afd80709',
    });

    server.use(http.get(ARCHIVE, () => new HttpResponse(stored, { status: 206 })));

    await expect(loadFile(fetch, BASE, CRATE, VERSION, file)).rejects.toMatchInlineSnapshot(
      `[Error: Integrity check failed for "src/lib.rs": SHA-256 mismatch]`,
    );
  });

  it('throws on an unsupported compression method', async () => {
    let bytes = encode('fn main() {}\n');
    let file = {
      ...manifestFile({
        compressed_size: bytes.length,
        uncompressed_size: bytes.length,
        sha256: await sha256Hex(bytes),
      }),
      compression: 'brotli',
    } as unknown as ManifestFile;

    server.use(http.get(ARCHIVE, () => new HttpResponse(bytes, { status: 206 })));

    await expect(loadFile(fetch, BASE, CRATE, VERSION, file)).rejects.toMatchInlineSnapshot(
      `[Error: Unsupported compression method "brotli"]`,
    );
  });

  it('reports a file containing a NUL byte as binary', async () => {
    let bytes = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x00, 0x00, 0x1a, 0x0a]);
    let file = manifestFile({
      path: 'icon.png',
      compression: 'store',
      compressed_size: bytes.length,
      uncompressed_size: bytes.length,
      sha256: await sha256Hex(bytes),
    });

    server.use(http.get(ARCHIVE, () => new HttpResponse(bytes, { status: 206 })));

    expect(await loadFile(fetch, BASE, CRATE, VERSION, file)).toEqual({ kind: 'binary' });
  });

  it('returns null when the CDN returns 404', async () => {
    server.use(http.get(ARCHIVE, () => new HttpResponse(null, { status: 404 })));
    expect(await loadFile(fetch, BASE, CRATE, VERSION, manifestFile())).toBeNull();
  });

  it('returns null when the CDN returns 403', async () => {
    let crate = await db.crate.create({ name: CRATE });
    await db.version.create({ crate, num: VERSION });

    expect(await loadFile(fetch, BASE, CRATE, VERSION, manifestFile())).toBeNull();
  });

  it('throws on any other error status', async () => {
    server.use(http.get(ARCHIVE, () => new HttpResponse(null, { status: 500 })));
    await expect(loadFile(fetch, BASE, CRATE, VERSION, manifestFile())).rejects.toMatchInlineSnapshot(
      `[Error: Failed to load file "src/lib.rs" (500 Internal Server Error)]`,
    );
  });
});
