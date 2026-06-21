import { describe, expect, test } from 'vitest';

import { db } from '../index.js';

async function inflateRaw(data) {
  let stream = new Blob([data]).stream().pipeThrough(new DecompressionStream('deflate-raw'));
  return new Uint8Array(await new Response(stream).arrayBuffer());
}

describe('GET /readmes/:name/:filename', () => {
  test('returns 403 for unknown crates', async function () {
    let response = await fetch('https://static.crates.io/readmes/foo/foo-1.0.0.html');
    expect(response.status).toBe(403);
    expect(await response.text()).toBe('');
  });

  test('returns 403 for unknown versions', async function () {
    await db.crate.create({ name: 'rand' });

    let response = await fetch('https://static.crates.io/readmes/rand/rand-1.0.0.html');
    expect(response.status).toBe(403);
    expect(await response.text()).toBe('');
  });

  test('returns 403 for versions without README', async function () {
    let crate = await db.crate.create({ name: 'rand' });
    await db.version.create({ crate, num: '1.0.0' });

    let response = await fetch('https://static.crates.io/readmes/rand/rand-1.0.0.html');
    expect(response.status).toBe(403);
    expect(await response.text()).toBe('');
  });

  test('returns the README as raw HTML', async function () {
    let readme = 'lorem ipsum <i>est</i> dolor!';

    let crate = await db.crate.create({ name: 'rand' });
    await db.version.create({ crate, num: '1.0.0', readme });

    let response = await fetch('https://static.crates.io/readmes/rand/rand-1.0.0.html');
    expect(response.status).toBe(200);
    expect(await response.text()).toBe(readme);
  });

  test('recovers the version for crate names containing dashes', async function () {
    let readme = 'serde readme';

    let crate = await db.crate.create({ name: 'serde-json' });
    await db.version.create({ crate, num: '1.0.0', readme });

    let response = await fetch('https://static.crates.io/readmes/serde-json/serde-json-1.0.0.html');
    expect(response.status).toBe(200);
    expect(await response.text()).toBe(readme);
  });

  test('decodes the version from the encoded filename', async function () {
    let readme = 'build metadata readme';

    let crate = await db.crate.create({ name: 'rand' });
    await db.version.create({ crate, num: '1.0.0+foo', readme });

    let response = await fetch('https://static.crates.io/readmes/rand/rand-1.0.0%2Bfoo.html');
    expect(response.status).toBe(200);
    expect(await response.text()).toBe(readme);
  });
});

describe('GET /crates/:name/:filename', () => {
  test('returns 403 for unknown crates', async function () {
    let response = await fetch('https://static.crates.io/crates/foo/foo-1.0.0.zip.json');
    expect(response.status).toBe(403);
  });

  test('returns 403 for a version without a source archive', async function () {
    let crate = await db.crate.create({ name: 'rand' });
    await db.version.create({ crate, num: '1.0.0' });

    let response = await fetch('https://static.crates.io/crates/rand/rand-1.0.0.zip.json');
    expect(response.status).toBe(403);
  });

  test('serves a manifest describing the source files', async function () {
    let crate = await db.crate.create({ name: 'rand' });
    await db.version.create({
      crate,
      num: '1.0.0',
      source_files: { 'src/lib.rs': 'fn main() {}\n', 'Cargo.toml': 'name = "rand"\n' },
    });

    let response = await fetch('https://static.crates.io/crates/rand/rand-1.0.0.zip.json');
    expect(response.status).toBe(200);

    let manifest = await response.json();
    expect(manifest).toMatchInlineSnapshot(`
      {
        "files": [
          {
            "compressed_size": 16,
            "compression": "deflate",
            "data_offset": 0,
            "path": "Cargo.toml",
            "sha256": "80eee9505e19fcad456f0d08ed5792ed10046f7042aa0adc87e37eb9a6bccf80",
            "uncompressed_size": 14,
          },
          {
            "compressed_size": 15,
            "compression": "deflate",
            "data_offset": 16,
            "path": "src/lib.rs",
            "sha256": "536e506bb90914c243a12b397b9a998f85ae2cbd9ba02dfd03a9e155ca5ca0f4",
            "uncompressed_size": 13,
          },
        ],
      }
    `);
  });

  test('serves a file via a range request that inflates to its contents', async function () {
    let content = 'fn main() { println!("hi"); }\n';

    let crate = await db.crate.create({ name: 'rand' });
    await db.version.create({ crate, num: '1.0.0', source_files: { 'src/lib.rs': content } });

    let manifestResponse = await fetch('https://static.crates.io/crates/rand/rand-1.0.0.zip.json');
    let manifest = await manifestResponse.json();
    let entry = manifest.files.find(file => file.path === 'src/lib.rs');

    let start = entry.data_offset;
    let end = entry.data_offset + entry.compressed_size - 1;
    let response = await fetch('https://static.crates.io/crates/rand/rand-1.0.0.zip', {
      headers: { Range: `bytes=${start}-${end}` },
    });
    expect(response.status).toBe(206);

    let compressed = new Uint8Array(await response.arrayBuffer());
    let text = new TextDecoder().decode(await inflateRaw(compressed));
    expect(text).toBe(content);
  });

  test('serves the whole archive when no range header is sent', async function () {
    let sourceFiles = { 'Cargo.toml': 'name = "rand"\n', 'src/lib.rs': 'fn main() {}\n' };

    let crate = await db.crate.create({ name: 'rand' });
    await db.version.create({ crate, num: '1.0.0', source_files: sourceFiles });

    let manifestResponse = await fetch('https://static.crates.io/crates/rand/rand-1.0.0.zip.json');
    let manifest = await manifestResponse.json();
    let entry = manifest.files.find(file => file.path === 'src/lib.rs');

    // Without a range header the whole archive comes back, and the client can slice
    // an entry out of it by its manifest offset.
    let response = await fetch('https://static.crates.io/crates/rand/rand-1.0.0.zip');
    expect(response.status).toBe(200);

    let body = new Uint8Array(await response.arrayBuffer());
    let compressed = body.slice(entry.data_offset, entry.data_offset + entry.compressed_size);
    let text = new TextDecoder().decode(await inflateRaw(compressed));
    expect(text).toBe('fn main() {}\n');
  });

  test('recovers the version when the crate name and version contain dashes', async function () {
    let crate = await db.crate.create({ name: 'serde-json' });
    await db.version.create({ crate, num: '1.0.0-beta.1', source_files: { 'Cargo.toml': 'name = "serde_json"\n' } });

    let response = await fetch('https://static.crates.io/crates/serde-json/serde-json-1.0.0-beta.1.zip.json');
    expect(response.status).toBe(200);
  });
});
