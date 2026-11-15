<script module lang="ts">
  import type { BlogPost } from '#lib/utils/blog.ts';
  import type { BlogPostsState } from './BlogPosts.svelte';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import BlogPosts from './BlogPosts.svelte';

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

  const NEW_POST: BlogPost = {
    title: 'crates.io: Trusted publishing for GitLab is now generally available',
    url: 'https://blog.rust-lang.org/2025/08/04/crates-io-gitlab-trusted-publishing/',
    published: new Date('2025-08-04T00:00:00Z'),
    isInsideRust: false,
  };

  const NOW = new Date('2025-08-07T12:00:00Z');

  const STATES = {
    Loaded: { status: 'loaded', posts: POSTS },
    'New Post': { status: 'loaded', posts: [NEW_POST, ...POSTS] },
    Loading: { status: 'loading' },
    Error: { status: 'error' },
    Empty: { status: 'loaded', posts: [] },
  } satisfies Record<string, BlogPostsState>;

  const { Story } = defineMeta({
    title: 'Frontpage/BlogPosts',
    component: BlogPosts,
    tags: ['autodocs'],
    render: template,
    argTypes: {
      state: { control: 'inline-radio', options: Object.keys(STATES) },
    },
    parameters: {
      controls: { exclude: ['now'] },
    },
  });
</script>

{#snippet template({ state }: { state: keyof typeof STATES })}
  <BlogPosts state={STATES[state]} now={NOW} />
{/snippet}

<Story name="Default" args={{ state: 'New Post' }} parameters={{ chromatic: { disableSnapshot: true } }} />

<!-- This is using a single Story with multiple examples to reduce the amount of snapshots generated for visual regression testing -->
<Story name="Combined" asChild>
  <h1>Loaded</h1>
  <BlogPosts state={STATES.Loaded} now={NOW} />

  <h1>New Post</h1>
  <BlogPosts state={STATES['New Post']} now={NOW} />

  <h1>Loading</h1>
  <BlogPosts state={STATES.Loading} />

  <h1>Error</h1>
  <BlogPosts state={STATES.Error} />

  <h1>Empty</h1>
  <BlogPosts state={STATES.Empty} />
</Story>

<style>
  h1 {
    font-size: 0.875rem;
    font-weight: normal;
    opacity: 0.2;
    margin: 1rem 0 0.25rem;
  }
</style>
