const PLAYGROUND_CRATES_URL = 'https://play.rust-lang.org/meta/crates';

/**
 * A crate available on the Rust Playground.
 *
 * The Rust Playground provides access to the top 100 most downloaded crates.
 */
export interface PlaygroundCrate {
  /**
   * The crate name as it appears on crates.io.
   * @example "aho-corasick"
   */
  name: string;
  /**
   * The version available on the playground.
   * @example "0.7.18"
   */
  version: string;
  /**
   * The module identifier used in Rust import statements.
   * @example "aho_corasick"
   */
  id: string;
}

/**
 * Loads the list of crates available on the Rust Playground.
 *
 * @throws Error if the request fails.
 */
export async function loadPlaygroundCrates(fetch: typeof globalThis.fetch): Promise<PlaygroundCrate[]> {
  let response = await fetch(PLAYGROUND_CRATES_URL, { priority: 'low' });

  if (!response.ok) {
    throw new Error('Failed to load Rust Playground crates');
  }

  let data: { crates: PlaygroundCrate[] } = await response.json();
  return data.crates;
}

/**
 * Builds a Rust Playground URL with pre-populated code for the given crate.
 *
 * @param id - The module identifier used in Rust import statements (e.g., "aho_corasick").
 * @returns A URL string to the Rust Playground with example code.
 */
export function buildPlaygroundLink(id: string): string {
  let code = `use ${id};\n\nfn main() {\n    // try using the \`${id}\` crate here\n}`;
  return `https://play.rust-lang.org/?edition=2021&code=${encodeURIComponent(code)}`;
}
