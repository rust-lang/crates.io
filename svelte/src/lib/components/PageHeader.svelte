<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  import LoadingSpinner from '$lib/components/LoadingSpinner.svelte';

  interface Props extends HTMLAttributes<HTMLDivElement> {
    title?: string;
    suffix?: string;
    showSpinner?: boolean;
    children?: Snippet;
  }

  let { title, suffix, showSpinner = false, children, class: className, ...others }: Props = $props();
</script>

<div data-test-page-header class={['header', className]} {...others}>
  {#if children}
    {@render children()}
  {:else}
    <h1 class="heading">
      {title}
      {#if suffix}
        <small class="suffix">{suffix}</small>
      {/if}
      {#if showSpinner}
        <LoadingSpinner style="margin-left: var(--space-2xs)" data-test-spinner />
      {/if}
    </h1>
  {/if}
</div>

<style>
  .header {
    padding: var(--space-s) var(--space-m);
    background-color: var(--main-bg-dark);
    margin-bottom: var(--space-s);
    border-radius: 5px;
  }

  .heading {
    display: flex;
    align-items: baseline;
    margin: 0;
  }

  .suffix {
    color: var(--main-color-light);
    padding-left: var(--space-2xs);
  }
</style>
