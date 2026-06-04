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
      '(Rust 1.70) and <code>LazyCell</code>/<code>LazyLock</code> (Rust 1.80). A few APIs (e.g. fallible ' +
      '<code>get_or_try_init</code>) are not yet stable in <code>std</code>.',
    url: 'https://blog.rust-lang.org/2024/07/25/Rust-1.80.0/',
  },

  matches: {
    description: 'The <code>matches!</code> macro has been part of the standard library since Rust 1.42.',
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
