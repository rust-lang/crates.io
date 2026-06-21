import type { ManifestFile } from '$lib/utils/zip-archive';

/** Files preferred as the initial selection, in priority order. */
const DEFAULT_FILE_PRIORITY = ['src/lib.rs', 'src/main.rs', 'Cargo.toml'];

/** Picks the file to redirect to, or `undefined` when the path is a file already. */
export function redirectTarget(files: ManifestFile[], path: string | undefined): ManifestFile | undefined {
  if (!path) {
    return pickDefaultFile(files);
  }

  if (files.some(file => file.path === path)) {
    return undefined;
  }

  // Not a file, so treat the path as a directory and jump to its first file.
  return firstFileInDirectory(files, path);
}

function pickDefaultFile(files: ManifestFile[]): ManifestFile | undefined {
  for (let path of DEFAULT_FILE_PRIORITY) {
    let match = files.find(file => file.path === path);
    if (match) {
      return match;
    }
  }

  // No preferred file exists, so fall back to the first file in the archive.
  return firstFile(files);
}

function firstFileInDirectory(files: ManifestFile[], dirPath: string): ManifestFile | undefined {
  let prefix = dirPath.endsWith('/') ? dirPath : `${dirPath}/`;
  return firstFile(files.filter(file => file.path.startsWith(prefix)));
}

/** Returns the file with the lexicographically smallest path. */
function firstFile(files: ManifestFile[]): ManifestFile | undefined {
  let first: ManifestFile | undefined;
  for (let file of files) {
    if (first === undefined || file.path < first.path) {
      first = file;
    }
  }
  return first;
}
