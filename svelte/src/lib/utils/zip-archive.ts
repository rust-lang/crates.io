/**
 * Utilities for reading published crate source files directly from the `.zip`
 * archives on the CDN, using the companion `.zip.json` manifest and HTTP range
 * requests.
 *
 * Each crate version is available as a seekable `.zip` archive at
 * `<base>/crates/<name>/<name>-<version>.zip`, with a manifest at the same URL
 * plus a `.json` suffix. The manifest lists every file along with the byte
 * offset and size of its compressed data inside the archive, which lets us fetch
 * and decode a single file without downloading the whole archive.
 */

/** The parsed contents of a `.zip.json` manifest. */
export interface Manifest {
  files: ManifestFile[];
}

/** A single file entry from the `.zip.json` manifest. */
export interface ManifestFile {
  /** Path of the file inside the archive, e.g. `src/lib.rs`. */
  path: string;
  /** Byte offset of the entry's compressed data within the `.zip` archive. */
  data_offset: number;
  /** Size of the compressed file in bytes. */
  compressed_size: number;
  /** Size of the decompressed file in bytes. */
  uncompressed_size: number;
  /** Compression method used for this entry. */
  compression: 'deflate' | 'store';
  /** Hex-encoded SHA-256 of the decompressed file contents. */
  sha256: string;
}

export type LoadedFile = { kind: 'text'; text: string } | { kind: 'binary' };

/** Returns the URL of the `.zip` archive for a crate version. */
export function archiveUrl(base: string, crateName: string, versionNum: string): string {
  return `${base}/crates/${crateName}/${crateName}-${versionNum}.zip`;
}

/** Returns the URL of the `.zip.json` manifest for a crate version. */
export function manifestUrl(base: string, crateName: string, versionNum: string): string {
  return `${archiveUrl(base, crateName, versionNum)}.json`;
}

/**
 * Loads and parses the `.zip.json` manifest for a crate version.
 *
 * @returns The parsed manifest, or `null` if no archive exists yet (404/403).
 * @throws Error If the request fails with any other non-success status.
 */
export async function loadManifest(
  fetch: typeof globalThis.fetch,
  base: string,
  crate: string,
  version: string,
): Promise<Manifest | null> {
  let response = await fetch(manifestUrl(base, crate, version));

  // The S3-backed CDN reports a not-yet-built archive as `404` or `403`.
  if (response.status === 404 || response.status === 403) {
    return null;
  }

  if (!response.ok) {
    throw new Error(`Failed to load archive manifest (${response.status} ${response.statusText})`);
  }

  return (await response.json()) as Manifest;
}

/**
 * Loads a single file from the archive via a range request.
 *
 * Only the bytes of the requested entry are fetched, then decompressed locally
 * using the browser's `DecompressionStream`. Binary files are detected and
 * reported as such instead of being decoded into mojibake.
 *
 * @returns The decoded text or a binary marker, or `null` if the archive is not
 *   available yet (404/403), mirroring {@link loadManifest}.
 * @throws Error If the request fails with any other non-success status.
 */
export async function loadFile(
  fetch: typeof globalThis.fetch,
  base: string,
  crateName: string,
  versionNum: string,
  file: ManifestFile,
): Promise<LoadedFile | null> {
  let start = file.data_offset;
  let end = file.data_offset + file.compressed_size - 1;

  let response = await fetch(archiveUrl(base, crateName, versionNum), {
    headers: { Range: `bytes=${start}-${end}` },
  });

  // The S3-backed CDN reports a not-yet-built archive as `404` or `403`.
  if (response.status === 404 || response.status === 403) {
    return null;
  }

  if (response.status !== 206 && response.status !== 200) {
    throw new Error(`Failed to load file "${file.path}" (${response.status} ${response.statusText})`);
  }

  let body = await response.arrayBuffer();
  let compressed =
    response.status === 206 ? body : body.slice(file.data_offset, file.data_offset + file.compressed_size);

  let bytes = await decompress(compressed, file.compression);
  await verifyIntegrity(bytes, file);

  if (looksBinary(bytes)) {
    return { kind: 'binary' };
  }

  return { kind: 'text', text: new TextDecoder().decode(bytes) };
}

/** Decompresses the raw entry bytes according to the manifest's method. */
async function decompress(compressed: ArrayBuffer, compression: ManifestFile['compression']): Promise<Uint8Array> {
  if (compression === 'store') {
    return new Uint8Array(compressed);
  }

  if (compression === 'deflate') {
    let stream = new Blob([compressed]).stream().pipeThrough(new DecompressionStream('deflate-raw'));
    return new Uint8Array(await new Response(stream).arrayBuffer());
  }

  throw new Error(`Unsupported compression method "${compression}"`);
}

/**
 * Verifies the decompressed bytes against the size and SHA-256 recorded in the
 * manifest, throwing if either differs.
 */
async function verifyIntegrity(bytes: Uint8Array, file: ManifestFile): Promise<void> {
  if (bytes.length !== file.uncompressed_size) {
    throw new Error(
      `Integrity check failed for "${file.path}": expected ${file.uncompressed_size} bytes, got ${bytes.length}`,
    );
  }

  let digest = await crypto.subtle.digest('SHA-256', bytes as BufferSource);
  let hex = [...new Uint8Array(digest)].map(byte => byte.toString(16).padStart(2, '0')).join('');
  if (hex !== file.sha256) {
    throw new Error(`Integrity check failed for "${file.path}": SHA-256 mismatch`);
  }
}

/**
 * Heuristic for whether a file is binary rather than displayable text: a NUL
 * byte within the first 8000 bytes, the same signal git uses.
 */
function looksBinary(bytes: Uint8Array): boolean {
  let limit = Math.min(bytes.length, 8000);
  for (let i = 0; i < limit; i++) {
    if (bytes[i] === 0) {
      return true;
    }
  }
  return false;
}
