/**
 * Native replacement policy
 * =========================
 *
 * This dataset flags crates whose functionality has been absorbed into the
 * Rust standard library, so a reader can see that `std` now provides what the
 * crate offers. Every entry is a checkable factual statement about `std`, never
 * an editorial judgement about a competing third-party crate.
 *
 * Scope
 * -----
 * Standard-library replacements only. The set deliberately does not include
 * "prefer this nicer crate instead" style recommendations.
 *
 * Inclusion criteria
 * ------------------
 * Two kinds of entries qualify:
 *
 *   - Full: the crate's functionality is entirely available via a stable `std`
 *     API.
 *   - Partial: the bulk of the crate's common use case has moved to `std`, but
 *     not all of it. The `description` must spell out what is still missing.
 *
 * Coverage is judged roughly, and only the dominant use case counts. If just a
 * small slice of a crate's purpose lives in `std`, it does not qualify, since
 * flagging it would be misleading and noisy. `itertools` is the canonical
 * exclusion: a few adaptors have `std` equivalents, but the crate is
 * overwhelmingly not replaced by `std`.
 *
 * Every entry's `description` must cite the stable `std` API(s) and the Rust
 * version(s) that stabilized them.
 *
 * Maintainer notice-and-comment
 * -----------------------------
 * Entries should arrive as PRs judged against the criteria above. Before an entry
 * lands, the maintainer(s) of the crate being flagged get a window to weigh in,
 * either to object or to show that the replacement is less complete than claimed,
 * in which case the entry becomes partial or is rejected.
 */

/** A crate whose functionality is (largely) available in std. */
export interface NativeReplacement {
  /** HTML fragment describing the std replacement. */
  description: string;
  /** Representative docs URL (std docs or release notes) shown as a "Learn more" link. */
  url: string;
}

export const nativeReplacements: Record<string, NativeReplacement> = {
  lazy_static: {
    description:
      'The standard library provides <code>std::sync::LazyLock</code> (stable since Rust 1.80), which lets ' +
      'you replace the <code>lazy_static!</code> macro with a plain <code>static</code> and remove the ' +
      'dependency.',
    url: 'https://doc.rust-lang.org/std/sync/struct.LazyLock.html',
  },

  once_cell: {
    description:
      'Most of <code>once_cell</code> is now in <code>std</code>: <code>OnceCell</code>/<code>OnceLock</code> ' +
      '(Rust 1.70) and <code>LazyCell</code>/<code>LazyLock</code> (Rust 1.80). The <code>race</code> module and ' +
      'fallible <code>get_or_try_init</code> have no stable <code>std</code> equivalent yet.',
    url: 'https://blog.rust-lang.org/2024/07/25/Rust-1.80.0/',
  },

  matches: {
    description:
      'The <code>matches!</code> macro has been in <code>std</code> since Rust 1.42, and ' +
      '<code>assert_matches!</code> / <code>debug_assert_matches!</code> followed in Rust 1.96.',
    url: 'https://doc.rust-lang.org/std/macro.matches.html',
  },

  num_cpus: {
    description:
      'For most uses, <code>std::thread::available_parallelism</code> (Rust 1.59) returns the parallelism ' +
      'available to the process. (It does not distinguish physical vs logical cores the way ' +
      '<code>num_cpus</code> does.)',
    url: 'https://doc.rust-lang.org/std/thread/fn.available_parallelism.html',
  },
};
