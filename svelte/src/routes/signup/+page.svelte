<script lang="ts">
  import { goto, invalidateAll } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { createClient } from '@crates-io/api-client';

  import Alert from '$lib/components/Alert.svelte';
  import PageTitle from '$lib/components/PageTitle.svelte';
  import { getSession } from '$lib/utils/session.svelte';

  const SIGNUP_ERROR_MESSAGE = 'Could not complete signup. Please try again.';
  const CANCEL_ERROR_MESSAGE = 'Could not cancel signup. Please try again.';

  let { data } = $props();
  let session = getSession();
  let client = createClient({ fetch });

  let emailOverride = $state<string | undefined>();
  let email = $derived(emailOverride ?? data.signup.email ?? '');

  let busy = $state(false);
  let errorMessage = $state('');

  let returnTo = $derived.by(() => {
    let url = page.url;
    try {
      let destination = new URL(url.searchParams.get('returnTo') || '/', url);
      if (destination.origin === url.origin) return destination;
    } catch {
      // Malformed destinations use the same fallback as external ones.
    }
    return new URL('/', url);
  });

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (busy) return;
    busy = true;
    errorMessage = '';

    try {
      let result = await client.POST('/api/private/session/signup', { body: { signup: { email } } });
      if (result.error) {
        let detail = result.error.errors[0]?.detail ?? SIGNUP_ERROR_MESSAGE;
        if (result.response.status === 400) {
          await invalidateAll();
        }
        errorMessage = detail;
        return;
      }

      await session.completeLogin();
      // eslint-disable-next-line svelte/no-navigation-without-resolve -- returnTo is validated against the current origin.
      await goto(returnTo, { invalidateAll: true });
    } catch {
      errorMessage = SIGNUP_ERROR_MESSAGE;
    } finally {
      busy = false;
    }
  }

  /** Clears pending signup before returning to the original page. */
  async function cancel() {
    if (busy) return;
    busy = true;
    errorMessage = '';
    try {
      let result = await client.DELETE('/api/private/session/signup');
      if (result.error) {
        errorMessage = result.error.errors[0]?.detail ?? CANCEL_ERROR_MESSAGE;
        return;
      }
      // eslint-disable-next-line svelte/no-navigation-without-resolve -- returnTo is validated against the current origin.
      await goto(returnTo);
    } catch {
      errorMessage = CANCEL_ERROR_MESSAGE;
    } finally {
      busy = false;
    }
  }
</script>

<PageTitle title="Create your crates.io account" />

<form onsubmit={submit} aria-busy={busy}>
  <h1>Create your crates.io account</h1>

  <div class="form-group mt-s">
    <label for="signup-username" class="form-group-name">Username</label>
    <input id="signup-username" class="base-input" value={data.signup.login} readonly />
  </div>

  <div class="form-group mt-s">
    <label for="signup-name" class="form-group-name">Display name</label>
    <input id="signup-name" class="base-input" value={data.signup.name ?? ''} readonly />
  </div>

  <div class="form-group mt-s">
    <label for="signup-email" class="form-group-name">Email address</label>
    <input
      id="signup-email"
      class="base-input"
      type="email"
      autocomplete="email"
      required
      bind:value={() => email, v => (emailOverride = v)}
      disabled={busy}
    />
  </div>

  <div class="agreement">
    <input id="signup-agreement" type="checkbox" required disabled={busy} aria-describedby="signup-agreement-warning" />
    <div>
      <label for="signup-agreement">
        I agree to the
        <a href={resolve('/policies')} target="_blank" rel="noopener noreferrer">Usage Policy</a>
        and
        <a href="https://foundation.rust-lang.org/policies/privacy-policy/" target="_blank" rel="noopener noreferrer">
          Privacy Policy
        </a>.
      </label>
      <p id="signup-agreement-warning">
        Violations of the Usage Policy may result in account suspension, termination, or removal of crates and releases.
      </p>
    </div>
  </div>

  {#if errorMessage}
    <Alert variant="caution" class="mt-s" role="alert">{errorMessage}</Alert>
  {/if}

  <div class="actions">
    <button type="submit" class="button button--small" disabled={busy}>Create account</button>
    <button type="button" class="button button--tan button--small" disabled={busy} onclick={cancel}>Cancel</button>
    {#if busy}<span role="status">Please wait…</span>{/if}
  </div>
</form>

<style>
  form {
    width: 100%;
    max-width: 28rem;
    margin: var(--space-l-xl) auto;
  }

  h1 {
    margin-top: 0;
    margin-bottom: var(--space-m);
  }

  .form-group input {
    width: 100%;
    min-width: 0;
  }

  input[readonly] {
    background-color: transparent;
  }

  .agreement {
    display: flex;
    align-items: baseline;
    gap: var(--space-2xs);
    margin-top: var(--space-m);
    line-height: 1.5;
  }

  .agreement p {
    margin: var(--space-s) 0 0;
    font-size: 0.85em;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-xs);
    margin-top: var(--space-m);
  }

  .actions button {
    border-radius: 4px;
  }
</style>
