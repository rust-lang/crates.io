<script lang="ts">
  import Icon from '$lib/components/Icon.svelte';
</script>

<div class="hero-buttons">
  <a
    href="https://doc.rust-lang.org/cargo/getting-started/installation.html"
    class="hero-button button"
    data-test-install-cargo-link
  >
    <Icon class="i-mdi:download" />
    Install Cargo
  </a>

  <a href="https://doc.rust-lang.org/cargo/guide/" class="hero-button button">
    <Icon class="i-mdi:flag-variant" />
    Getting Started
  </a>
</div>

<style>
  .hero-buttons {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: var(--space-s);
    border-bottom: 5px solid var(--gray-border);
    margin-top: var(--space-s);
    padding-bottom: var(--space-l);
  }

  .hero-button {
    --icon-size: 1.5em;
  }

  .hero-button :global(.icon) {
    color: #c4890e;
    margin: -0.25em;
    margin-right: var(--space-3xs);
  }
</style>
