<script module lang="ts">
  import type { BlogPost } from '#lib/utils/blog.ts';

  export type BlogPostsState = { status: 'loading' } | { status: 'error' } | { status: 'loaded'; posts: BlogPost[] };
</script>

<script lang="ts">
  import Icon from '#lib/components/Icon.svelte';
  import Placeholder from '#lib/components/Placeholder.svelte';

  const TAG_URL = 'https://blog.rust-lang.org/tags/crates-io/';

  // Blog posts are dated by day at midnight UTC.
  const DATE_FORMAT = new Intl.DateTimeFormat(undefined, { dateStyle: 'medium', timeZone: 'UTC' });

  const PLACEHOLDER_TITLE_WIDTHS = ['55%', '70%', '45%'];

  let { state }: { state: BlogPostsState } = $props();

  let posts = $derived(state.status === 'loaded' ? state.posts.slice(0, 3) : []);
</script>

<section class="blog" data-test-blog-posts>
  <header>
    <h2>News</h2>
    <a class="all" href={TAG_URL}>All posts on the Rust blog <Icon class="i-mdi:chevron-right" /></a>
  </header>
  <div class="panel">
    {#if state.status === 'loading'}
      <ul aria-busy="true">
        {#each PLACEHOLDER_TITLE_WIDTHS as width (width)}
          <li class="post-placeholder" data-test-blog-post-placeholder>
            <Placeholder width="85px" height="12px" radius="6px" opacity={0.2} />
            <Placeholder {width} height="14px" radius="7px" opacity={0.25} />
          </li>
        {/each}
      </ul>
    {:else if posts.length === 0}
      <p class="error" data-test-blog-posts-error>
        <span>The latest posts could not be loaded. You can find them on the <a href={TAG_URL}>Rust blog</a>.</span>
      </p>
    {:else}
      <ul>
        {#each posts as post (post.url)}
          <li>
            <a class="post" href={post.url} target="_blank" rel="external" data-test-blog-post>
              <span class="title">
                <span class="text" data-test-title>{post.title}</span>
                {#if post.isInsideRust}
                  <span class="badge" data-test-inside-rust-badge>Inside Rust</span>
                {/if}
              </span>
              <time datetime={post.published.toISOString()}>{DATE_FORMAT.format(post.published)}</time>
            </a>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</section>

<style>
  .blog {
    --row-height: 2.5rem;
    --row-padding: var(--space-s);
    --date-min-width: calc(7 * var(--list-row-metadata-font-size));
    --placeholder-bg: light-dark(hsla(59, 19%, 50%, 1), hsl(60, 14%, 85%));
    --placeholder-bg2: light-dark(hsla(59, 19%, 50%, 0.7), hsla(59, 5%, 50%, 0.7));
    margin: 0 var(--space-s) var(--space-m);

    @media only screen and (max-width: 550px) {
      --row-height: 4.2rem;
      --row-padding: var(--space-m);
    }
  }

  header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
  }

  h2 {
    font-size: 1.05rem;
  }

  .all {
    display: flex;
    align-items: center;
    min-height: 2rem;
    margin-block: calc(-1 * var(--space-2xs));
    margin-right: calc(-1 * var(--space-2xs));
    padding: var(--space-2xs);
    font-size: var(--list-row-metadata-font-size);
    color: var(--main-color-light);

    &:hover {
      color: var(--link-hover-color);
    }
  }

  .panel {
    /* Reserves three rows and their separators in every state to avoid layout shifts. */
    min-height: calc(3 * var(--row-height) + 2px);
    background: var(--surface);
    border-radius: var(--border-radius-small);
    box-shadow: var(--surface-shadow-muted);
  }

  ul {
    display: grid;
    grid-template-columns: minmax(calc(var(--date-min-width) + var(--row-padding)), max-content) minmax(0, 1fr);
    column-gap: var(--space-s);
    list-style: none;
    padding: 0;
    margin: 0;
  }

  li,
  .post {
    grid-column: 1 / -1;
    display: grid;
    grid-template-columns: subgrid;
  }

  li + li {
    border-top: 1px solid var(--divider-color-muted);
  }

  .post,
  .post-placeholder {
    align-items: center;
    min-height: var(--row-height);
    padding: 0 var(--row-padding);
  }

  .post {
    color: var(--main-color);
    transition: background-color var(--transition-slow);

    &:hover {
      background-color: light-dark(hsl(58deg 72% 97%), hsl(204, 3%, 11%));
      transition: background-color var(--transition-instant);
    }

    &:focus-visible {
      outline: none;
      box-shadow: inset 0 0 0 3px var(--focus-ring-color);
    }
  }

  li:first-child > .post {
    border-top-left-radius: var(--border-radius-small);
    border-top-right-radius: var(--border-radius-small);
  }

  li:last-child > .post {
    border-bottom-left-radius: var(--border-radius-small);
    border-bottom-right-radius: var(--border-radius-small);
  }

  time {
    /* The title comes first in the link name, but the date is displayed first. */
    order: -1;
    text-align: right;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
    font-size: var(--list-row-metadata-font-size);
    color: var(--main-color-light);
  }

  .title {
    display: flex;
    align-items: center;
    gap: 0.6em;
    min-width: 0;
  }

  .text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .badge {
    flex-shrink: 0;
    padding: 0.25em 0.5em 0.2em;
    font-size: 0.6rem;
    font-weight: 600;
    letter-spacing: 0.1em;
    line-height: 1.2;
    text-transform: uppercase;
    white-space: nowrap;
    color: var(--main-color-light);
    border: 1px solid currentColor;
    border-radius: 3px;
  }

  .error {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: inherit;
    margin: 0;
    padding: 0 var(--row-padding);
    text-align: center;
    font-size: var(--list-row-metadata-font-size);
    color: var(--main-color-light);
  }

  /* On phones the date and badge share the first line and the title moves below them. */
  @media only screen and (max-width: 550px) {
    ul,
    li {
      display: block;
    }

    .post,
    .post-placeholder {
      display: flex;
      flex-wrap: wrap;
      align-content: center;
      gap: var(--space-3xs) var(--space-2xs);
      padding-block: var(--space-xs);
    }

    .post-placeholder {
      flex-flow: column nowrap;
      align-items: flex-start;
      justify-content: center;
    }

    .title {
      display: contents;
    }

    .text {
      order: 1;
      width: 100%;
    }
  }
</style>
