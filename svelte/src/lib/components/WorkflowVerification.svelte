<script lang="ts">
  import { untrack } from 'svelte';

  interface Props {
    url: string;
    fieldType?: string;
  }

  let { url, fieldType = 'filename' }: Props = $props();

  type Status = 'initial' | 'running' | Result;
  type Result = 'success' | 'not-found' | 'error';

  let result = $state<Result | undefined>(undefined);
  let isRunning = $state(false);
  let currentController: AbortController | undefined;

  let status: Status = $derived(isRunning ? 'running' : result ? result : 'initial');

  let isSuccess = $derived(result === 'success');
  let isWarning = $derived(result === 'not-found' || result === 'error');

  $effect(() => {
    // Capture `url` as the only tracked dependency, then run the
    // verification logic inside `untrack()` to avoid infinite loops
    // caused by writing to `result` and `isRunning` state.
    let currentUrl = url;
    return untrack(() => scheduleVerification(currentUrl));
  });

  function scheduleVerification(targetUrl: string): (() => void) | undefined {
    // Reset state when the URL is empty
    if (!targetUrl) {
      result = undefined;
      isRunning = false;
      return;
    }

    // Cancel any in-flight verification
    currentController?.abort();

    let controller = new AbortController();
    currentController = controller;

    isRunning = true;
    result = undefined;

    // Debounce the HEAD request by 500ms to avoid firing on every keystroke
    let timeout = setTimeout(async () => {
      if (controller.signal.aborted) return;

      let value = await verify(targetUrl);
      if (!controller.signal.aborted) {
        result = value;
        isRunning = false;
      }
    }, 500);

    return () => {
      clearTimeout(timeout);
      controller.abort();
    };
  }

  async function verify(targetUrl: string): Promise<Result> {
    try {
      let response = await fetch(targetUrl, { method: 'HEAD' });

      if (response.ok) {
        return 'success';
      } else if (response.status === 404) {
        return 'not-found';
      } else {
        return 'error';
      }
    } catch {
      return 'error';
    }
  }
</script>

<div
  class="workflow-verification"
  class:workflow-verification--success={isSuccess}
  class:workflow-verification--warning={isWarning}
  data-test-workflow-verification={status}
>
  {#if status === 'running'}
    Verifying...
  {:else if status === 'success'}
    ✓ Workflow file found at
    <!-- eslint-disable svelte/no-navigation-without-resolve -->
    <a href={url} target="_blank" rel="noopener noreferrer">{url}</a>
  {:else if status === 'not-found'}
    ⚠ Workflow file not found at
    <!-- eslint-disable svelte/no-navigation-without-resolve -->
    <a href={url} target="_blank" rel="noopener noreferrer">{url}</a>
  {:else if status === 'error'}
    ⚠ Could not verify workflow file at
    <!-- eslint-disable svelte/no-navigation-without-resolve -->
    <a href={url} target="_blank" rel="noopener noreferrer">{url}</a>
    (network error)
  {:else}
    The workflow {fieldType} will be verified once all necessary fields are filled.
  {/if}
</div>

<style>
  .workflow-verification {
    margin-top: var(--space-2xs);
    font-size: 0.85em;

    a,
    a:hover {
      color: inherit;
    }
  }

  .workflow-verification--success {
    color: var(--green800);
  }

  .workflow-verification--warning {
    color: var(--yellow700);
  }
</style>
