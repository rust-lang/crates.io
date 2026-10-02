<script lang="ts">
  import { getPageTitle } from '#lib/page-title.svelte.ts';

  let { title }: { title: string } = $props();

  let pageTitle = getPageTitle();

  $effect(() => {
    return pageTitle.push(title);
  });
</script>
