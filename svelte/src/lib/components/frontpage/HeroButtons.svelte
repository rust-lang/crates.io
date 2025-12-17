<script lang="ts">
  import DownloadArrowIcon from '$lib/assets/download-arrow.svg?component';
  import FlagIcon from '$lib/assets/flag.svg?component';
</script>

<div class="hero-buttons">
  <a
    href="https://doc.rust-lang.org/cargo/getting-started/installation.html"
    class="hero-button button"
    data-test-install-cargo-link
  >
    <DownloadArrowIcon class="icon" />
    Install Cargo
  </a>

  <a href="https://doc.rust-lang.org/cargo/guide/" class="hero-button button">
    <FlagIcon class="icon" />
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

  .hero-button :global(.icon) {
    color: #c4890e;
  }
</style>
