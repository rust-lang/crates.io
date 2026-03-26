<script lang="ts">
  import { page } from '$app/state';

  import Ferris from '$lib/assets/cuddlyferris.svg?url';
  import { getSession } from '$lib/utils/session.svelte';

  let session = getSession();

  function errorMessage(page: { status: number; error: { message: string } | null }) {
    if (page.status === 404 && page.error?.message === 'Not Found') {
      return 'Page not found';
    }
    return page.error?.message ?? 'Page not found';
  }

  function goBack() {
    history.back();
  }

  function reload() {
    location.reload();
  }
</script>

<div class="wrapper" data-test-404-page>
  <div class="content">
    <img src={Ferris} alt="" class="logo" />

    <h1 class="title" data-test-title>
      {errorMessage(page)}
    </h1>

    {#if page.error?.details}
      <p class="details" data-test-details>{page.error.details}</p>
    {/if}

    {#if page.error?.loginNeeded}
      <button
        type="button"
        class="link button-reset text--link"
        data-test-login
        disabled={session.state === 'logging-in'}
        onclick={() => session.login()}
      >
        Log in with GitHub
      </button>
    {:else if page.error?.tryAgain}
      <button type="button" class="link button-reset text--link" data-test-try-again onclick={reload}>Try Again</button>
    {:else}
      <button type="button" class="link button-reset text--link" data-test-go-back onclick={goBack}>Go Back</button>
    {/if}
  </div>
</div>

<style>
  .wrapper {
    height: 100%;
    display: grid;
    place-items: center;
  }

  .content {
    display: grid;
    place-items: center;
    margin: var(--space-m) 0;
  }

  .logo {
    max-width: 200px;
  }

  .link {
    font-weight: 500;

    &[disabled] {
      color: var(--grey600);
      cursor: wait;
    }
  }
</style>
