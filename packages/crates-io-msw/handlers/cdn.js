import { http, HttpResponse } from 'msw';

import { db } from '../index.js';

const ENCODER = new TextEncoder();

export default [
  http.get('https://static.crates.io/readmes/:name/:filename', async ({ params }) => {
    let crate = db.crate.findFirst(q => q.where({ name: params.name }));
    if (!crate) return HttpResponse.html('', { status: 403 });

    // The expected filename is `${name}-${version}.html`. This recovers the version
    // and decodes the `%2B` applied to versions with build metadata.
    let version = decodeURIComponent(params.filename.slice(`${params.name}-`.length, -'.html'.length));

    let found = db.version.findFirst(q => q.where(v => v.crate.id === crate.id && v.num === version));
    if (!found || !found.readme) return HttpResponse.html('', { status: 403 });

    return HttpResponse.html(found.readme);
  }),

  http.get('https://static.crates.io/crates/:name/:filename', async ({ params, request }) => {
    let { name, filename } = params;

    let suffix = filename.endsWith('.zip.json') ? '.zip.json' : filename.endsWith('.zip') ? '.zip' : null;
    if (!suffix) return;

    // The expected filename is `${name}-${version}${suffix}`. This recovers the
    // version and decodes the `%2B` applied to versions with build metadata.
    let version = decodeURIComponent(filename.slice(`${name}-`.length, -suffix.length));

    let sourceFiles = findSourceFiles(name, version);
    if (!sourceFiles) {
      // The S3-backed CDN reports a not-yet-built archive as `403`.
      return new HttpResponse(null, { status: 403 });
    }

    let { body, manifest } = await buildArchive(sourceFiles);

    if (suffix === '.zip.json') {
      return HttpResponse.json(manifest);
    }

    let range = request.headers.get('Range');
    if (range) {
      let slice = sliceRange(body, range);
      if (slice) {
        return new HttpResponse(slice, { status: 206 });
      }
    }

    return new HttpResponse(body, { status: 200 });
  }),
];

/** Looks up the `source_files` of a crate version, or `undefined` if there are none. */
function findSourceFiles(name, version) {
  let crate = db.crate.findFirst(q => q.where({ name }));
  if (!crate) return;

  let found = db.version.findFirst(q => q.where(v => v.crate.id === crate.id && v.num === version));
  return found?.source_files ?? undefined;
}

/**
 * Builds a seekable archive body and its manifest from a map of source files.
 *
 * The client locates each entry purely by the manifest's byte offsets plus a
 * range request, so the body is a plain concatenation of the compressed entries
 * rather than a real ZIP container, which is good enough for our purposes.
 */
async function buildArchive(sourceFiles) {
  let files = [];
  let chunks = [];
  let offset = 0;

  for (let path of Object.keys(sourceFiles).toSorted()) {
    let uncompressed = ENCODER.encode(sourceFiles[path]);
    let compressed = await deflateRaw(uncompressed);
    files.push({
      path,
      data_offset: offset,
      compressed_size: compressed.length,
      uncompressed_size: uncompressed.length,
      compression: 'deflate',
      sha256: await sha256Hex(uncompressed),
    });
    chunks.push(compressed);
    offset += compressed.length;
  }

  let body = new Uint8Array(offset);
  let position = 0;
  for (let chunk of chunks) {
    body.set(chunk, position);
    position += chunk.length;
  }

  return { body, manifest: { files } };
}

/** Compresses bytes as a raw `DEFLATE` stream, matching how zip entries are stored. */
async function deflateRaw(data) {
  let stream = new Blob([data]).stream().pipeThrough(new CompressionStream('deflate-raw'));
  return new Uint8Array(await new Response(stream).arrayBuffer());
}

/** Returns the hex-encoded SHA-256 digest of the given bytes. */
async function sha256Hex(data) {
  let digest = await crypto.subtle.digest('SHA-256', data);
  return [...new Uint8Array(digest)].map(byte => byte.toString(16).padStart(2, '0')).join('');
}

/** Slices a `bytes=<start>-<end>` range out of the body, or `null` if malformed. */
function sliceRange(body, range) {
  let match = /^bytes=(\d+)-(\d+)$/.exec(range);
  if (!match) return null;
  return body.slice(Number(match[1]), Number(match[2]) + 1);
}
