<script module lang="ts">
  import type { BlogPost } from '#lib/utils/blog.ts';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import BlogPosts from './BlogPosts.svelte';

  const { Story } = defineMeta({
    title: 'Frontpage/BlogPosts',
    component: BlogPosts,
    tags: ['autodocs'],
  });

  const POSTS: BlogPost[] = [
    {
      title: 'crates.io: development update',
      url: 'https://blog.rust-lang.org/2025/07/11/crates-io-development-update/',
      published: new Date('2025-07-11T00:00:00Z'),
      isInsideRust: false,
    },
    {
      title: 'The new crates.io frontend',
      url: 'https://blog.rust-lang.org/inside-rust/2025/06/20/crates-io-new-frontend/',
      published: new Date('2025-06-20T00:00:00Z'),
      isInsideRust: true,
    },
    {
      title: 'crates.io phishing campaign and malicious crates targeting Rust developers in the wild',
      url: 'https://blog.rust-lang.org/2025/05/24/crates-io-malicious-crates/',
      published: new Date('2025-05-24T00:00:00Z'),
      isInsideRust: false,
    },
  ];
</script>

<Story name="Loaded" args={{ state: { status: 'loaded', posts: POSTS } }} />

<Story name="Loading" args={{ state: { status: 'loading' } }} />

<Story name="Error" args={{ state: { status: 'error' } }} />

<Story name="Empty" args={{ state: { status: 'loaded', posts: [] } }} />
