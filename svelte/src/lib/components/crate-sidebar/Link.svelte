<script module lang="ts">
  export function simplifyUrl(url: string): string {
    if (url.startsWith('https://')) {
      url = url.slice('https://'.length);
    }
    if (url.startsWith('www.')) {
      url = url.slice('www.'.length);
    }
    if (url.endsWith('/')) {
      url = url.slice(0, -1);
    }
    if (url.startsWith('github.com/') && url.endsWith('.git')) {
      url = url.slice(0, -4);
    }

    return url;
  }
</script>

<script lang="ts">
  import type { HTMLAttributes } from 'svelte/elements';

  import CodebergIcon from '$lib/assets/codeberg.svg?component';
  import DocsRsIcon from '$lib/assets/docs-rs.svg?component';
  import GitHubIcon from '$lib/assets/github.svg?component';
  import GitLabIcon from '$lib/assets/gitlab.svg?component';
  import LinkIcon from '$lib/assets/link.svg?component';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    title: string;
    url: string;
  }

  let { url, title, ...restProps }: Props = $props();

  let text = $derived(simplifyUrl(url));
</script>

<div {...restProps}>
  <h2 class="title" data-test-title>{title}</h2>
  <div class="content">
    {#if text.startsWith('docs.rs/')}
      <DocsRsIcon class="icon" data-test-icon="docs-rs" />
    {:else if text.startsWith('github.com/')}
      <GitHubIcon class="icon" data-test-icon="github" />
    {:else if text.startsWith('gitlab.com/')}
      <GitLabIcon class="icon" data-test-icon="gitlab" />
    {:else if text.startsWith('codeberg.org/')}
      <CodebergIcon class="icon" data-test-icon="codeberg" />
    {:else}
      <LinkIcon class="icon" data-test-icon="link" />
    {/if}

    <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
    <a href={url} class="link" data-test-link>{text}</a>
  </div>
</div>

<style>
  .content {
    display: flex;
    align-items: center;
  }

  .title {
    font-size: 1.17em;
    margin: 0 0 var(--space-s);
  }

  .content :global(.icon) {
    flex-shrink: 0;
    height: 1em;
    width: auto;
    margin-right: var(--space-2xs);
  }

  .link {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
