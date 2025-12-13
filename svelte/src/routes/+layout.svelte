<script lang="ts">
  import '$lib/css/global.css';
  import favicon from '$lib/assets/favicon.svg';

  import Header from '$lib/components/Header.svelte';

  // TODO: import ProgressBar from '$lib/components/ProgressBar.svelte';
  // TODO: import Footer from '$lib/components/Footer.svelte';

  let { children } = $props();

  // TODO: implement color scheme support
  // TODO: implement notification container
</script>

<svelte:head>
  <link rel="icon" href={favicon} />
  <title>crates.io: Rust Package Registry</title>
</svelte:head>

<!-- TODO: <ProgressBar /> -->
<!-- TODO: <NotificationContainer position='top-right' /> -->
<div id="tooltip-container"></div>

<!-- TODO: pass `hero` prop based on whether we're on the index route -->
<Header />

<main class="main">
  <div class="inner-main width-limit">
    {@render children()}
  </div>
</main>

<!-- TODO: <Footer /> -->

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
