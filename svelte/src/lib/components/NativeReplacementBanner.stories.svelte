<script module lang="ts">
  import { defineMeta } from '@storybook/addon-svelte-csf';

  import NativeReplacementBanner from './NativeReplacementBanner.svelte';

  const { Story } = defineMeta({
    title: 'NativeReplacementBanner',
    component: NativeReplacementBanner,
    tags: ['autodocs'],
  });

  const full = {
    description:
      'The standard library provides `std::sync::LazyLock` (stable since Rust 1.80), which lets you replace the `lazy_static!` macro with a plain `static` and remove the dependency.',
    url: 'https://doc.rust-lang.org/std/sync/struct.LazyLock.html',
  };

  const fullMacro = {
    description:
      'The `matches!` macro has been in `std` since Rust 1.42, and `assert_matches!` / `debug_assert_matches!` followed in Rust 1.96.',
    url: 'https://doc.rust-lang.org/std/macro.matches.html',
  };

  const partial = {
    description:
      'For most uses, `std::thread::available_parallelism` (Rust 1.59) returns the parallelism available to the process. (It does not distinguish physical vs logical cores the way `num_cpus` does.)',
    url: 'https://doc.rust-lang.org/std/thread/fn.available_parallelism.html',
  };

  const partialReleaseNotes = {
    description:
      'Most of `once_cell` is now in `std`: `OnceCell`/`OnceLock` (Rust 1.70) and `LazyCell`/`LazyLock` (Rust 1.80). The `race` module and fallible `get_or_try_init` have no stable `std` equivalent yet.',
    url: 'https://blog.rust-lang.org/2024/07/25/Rust-1.80.0/',
  };
</script>

<Story name="Default" args={{ replacement: full }} parameters={{ chromatic: { disableSnapshot: true } }} />

<!-- This is using a single Story with multiple examples to reduce the amount of snapshots generated for visual regression testing -->
<Story name="Combined" asChild>
  <h1>Full</h1>
  <NativeReplacementBanner replacement={full} />

  <h1>Full (macro)</h1>
  <NativeReplacementBanner replacement={fullMacro} />

  <h1>Partial</h1>
  <NativeReplacementBanner replacement={partial} />

  <h1>Partial (release-notes link)</h1>
  <NativeReplacementBanner replacement={partialReleaseNotes} />
</Story>

<style>
  h1 {
    font-size: 0.875rem;
    font-weight: normal;
    opacity: 0.2;
    margin: 1rem 0 0.25rem;
  }
</style>
