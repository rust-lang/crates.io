<script lang="ts">
  import { getProgressContext } from '$lib/progress.svelte';

  let progress = getProgressContext();
</script>

<div class="progress-bar" style={progress.style}></div>

<style>
  .progress-bar {
    position: fixed;
    left: 0;
    top: 0;
    height: 3px;
    box-shadow: 0 0 10px rgba(0, 13, 41, 0.6);
    background: var(--yellow500);
  }
</style>
