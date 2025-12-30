<script lang="ts">
  import { page } from '$app/state';

  import { ColorSchemeState, setColorScheme } from '$lib/color-scheme.svelte';
  import Footer from '$lib/components/Footer.svelte';
  import Header from '$lib/components/Header.svelte';

  import '$lib/css/global.css';

  // TODO: import ProgressBar from '$lib/components/ProgressBar.svelte';

  let { children } = $props();

  let isIndex = $derived(page.route.id === '/');

  let colorScheme = new ColorSchemeState();
  setColorScheme(colorScheme);

  $effect(() => {
    document.documentElement.dataset.colorScheme = colorScheme.resolvedScheme;
  });

  // TODO: implement notification container
</script>

<svelte:head>
  <title>crates.io: Rust Package Registry</title>
</svelte:head>

<!-- TODO: <ProgressBar /> -->
<!-- TODO: <NotificationContainer position='top-right' /> -->
<div id="tooltip-container"></div>

<Header hero={isIndex} />

<main class="main">
  <div class="inner-main width-limit">
    {@render children()}
  </div>
</main>

<Footer />

<style>
  .main {
    flex-grow: 1;
    display: flex;
    justify-content: center;
    width: 100%;
    position: relative;
    background-color: var(--main-bg);
    color: var(--main-color);
    box-shadow: 0 0 6px 0 var(--main-shadow-color);
  }

  .inner-main {
    --main-layout-padding: var(--space-s);

    display: flex;
    flex-direction: column;
    padding: var(--main-layout-padding);
  }
</style>
