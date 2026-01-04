<script lang="ts">
  import { parseLicense } from '$lib/utils/license';

  interface Props {
    license: string;
  }

  let { license }: Props = $props();

  let parts = $derived(parseLicense(license));
</script>

{#each parts as part (part)}
  {#if part.isKeyword}
    <small>{` ${part.text} `}</small>
  {:else if part.link}
    <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
    <a href={part.link} rel="noreferrer">{part.text}</a>
  {:else}
    {part.text}
  {/if}
{/each}
