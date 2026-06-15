// The dataset and its inclusion policy live at
// https://github.com/rust-lang/std-replacement-data.

/** A crate whose functionality is (largely) available in std. */
export interface NativeReplacement {
  /** Markdown describing the std replacement. */
  description: string;
  /** Representative docs URL (std docs or release notes) shown as a "Learn more" link. */
  url: string;
}

/** The native-replacement dataset, keyed by crate name. */
export type NativeReplacements = Record<string, NativeReplacement>;

const DATA_URL = 'https://rust-lang.github.io/std-replacement-data/all.json';

/** Maximum time a single {@link loadNativeReplacements} call waits for the data. */
const TIMEOUT_MS = 5000;

let cache: Promise<NativeReplacements> | undefined;

/**
 * Loads the native-replacement dataset from the upstream repository.
 *
 * The data is optional, so this never rejects: on a network error, a non-2xx
 * response, or invalid JSON it logs a warning and resolves to an empty map.
 *
 * The fetch runs at most once per session and its result is cached. A failed
 * fetch clears the cache so a later call retries. Each call gives up after
 * {@link TIMEOUT_MS} to avoid blocking rendering, but the underlying request is
 * not aborted: if it completes later, it warms the cache for the next call.
 */
export function loadNativeReplacements(fetch: typeof globalThis.fetch): Promise<NativeReplacements> {
  cache ??= fetchNativeReplacements(fetch);

  let timeoutId: ReturnType<typeof setTimeout>;
  let timeout = new Promise<NativeReplacements>(resolve => {
    timeoutId = setTimeout(() => resolve({}), TIMEOUT_MS);
  });

  return Promise.race([cache, timeout]).finally(() => clearTimeout(timeoutId));
}

async function fetchNativeReplacements(fetch: typeof globalThis.fetch): Promise<NativeReplacements> {
  try {
    let response = await fetch(DATA_URL, { priority: 'low' });
    if (!response.ok) {
      throw new Error(`Unexpected response status: ${response.status}`);
    }
    return await response.json();
  } catch (error) {
    console.warn(`Failed to load native-replacement data: ${error}`);
    cache = undefined;
    return {};
  }
}
