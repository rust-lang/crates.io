import { createContext, untrack } from 'svelte';

const BASE_TITLE = 'crates.io: Rust Package Registry';
const SEPARATOR = ' - ';

/**
 * Wrapper object so cleanup can use object identity to remove the
 * correct token, even when multiple segments have the same string value.
 */
interface TitleToken {
  title: string;
}

/**
 * Manages a composable stack of page title segments.
 *
 * Title segments are pushed by nested components (layouts, pages) and
 * composed into a single document title. The most recently pushed
 * segment appears first, followed by parent segments, followed by
 * the base title.
 *
 * Example: pushing "serde" then "Versions" produces
 * "Versions - serde - crates.io: Rust Package Registry"
 */
export class PageTitleState {
  /**
   * Stack of active title segments, ordered from outermost (layout) to
   * innermost (page). Uses `$state.raw` instead of `$state` so that
   * token objects are not wrapped in proxies, which would break the
   * identity comparison in the cleanup function returned by {@link push}.
   */
  #tokens = $state.raw<TitleToken[]>([]);

  /** The composed document title string. */
  get title(): string {
    if (this.#tokens.length === 0) return BASE_TITLE;

    let segments = this.#tokens.map(t => t.title).reverse();
    return [...segments, BASE_TITLE].join(SEPARATOR);
  }

  /**
   * Pushes a title segment onto the stack.
   *
   * Returns a cleanup function that removes the segment. This is
   * designed to be used as the return value of a `$effect()` callback,
   * so the segment is automatically removed when the component unmounts
   * or the effect re-runs with a new value.
   */
  push(title: string): () => void {
    let token: TitleToken = { title };
    this.#tokens = [...untrack(() => this.#tokens), token];

    return () => {
      this.#tokens = untrack(() => this.#tokens).filter(t => t !== token);
    };
  }
}

export const [getPageTitle, setPageTitle] = createContext<PageTitleState>();
