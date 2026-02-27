<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { createClient } from '@crates-io/api-client';

  import CircleQuestionIcon from '$lib/assets/circle-question.svg?component';
  import TrashIcon from '$lib/assets/trash.svg?component';
  import LoadingSpinner from '$lib/components/LoadingSpinner.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import PatternDescription from '$lib/components/PatternDescription.svelte';
  import SettingsPage from '$lib/components/SettingsPage.svelte';
  import { getNotifications } from '$lib/notifications.svelte';
  import { scopeDescription } from '$lib/utils/token-scopes';
  import { getTokenPageState } from '../+layout.svelte';

  const ENDPOINT_SCOPES = ['change-owners', 'publish-new', 'publish-update', 'trusted-publishing', 'yank'];

  let notifications = getNotifications();
  let client = createClient({ fetch });
  let id = $props.id();
  let tokenPageState = getTokenPageState();

  class CratePattern {
    pattern = $state('');
    showAsInvalid = $state(false);

    constructor(pattern: string) {
      this.pattern = pattern;
    }

    get isValid(): boolean {
      return isValidPattern(this.pattern);
    }
  }

  function isValidPattern(pattern: string): boolean {
    if (!pattern) return false;
    if (pattern === '*') return true;

    if (pattern.endsWith('*')) {
      pattern = pattern.slice(0, -1);
    }

    return isValidIdent(pattern);
  }

  function isValidIdent(pattern: string): boolean {
    return (
      [...pattern].every(c => isAsciiAlphanumeric(c) || c === '_' || c === '-') &&
      pattern[0] !== '_' &&
      pattern[0] !== '-'
    );
  }

  function isAsciiAlphanumeric(c: string): boolean {
    return (c >= '0' && c <= '9') || (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z');
  }

  let name = $state('');
  let nameInvalid = $state(false);
  let expirySelection = $state('90');
  let expiryDateInput = $state('');
  let expiryDateInvalid = $state(false);
  let scopes = $state<string[]>([]);
  let scopesInvalid = $state(false);
  let crateScopes = $state<CratePattern[]>([]);
  let isSaving = $state(false);

  let today = $derived(new Date().toISOString().slice(0, 10));

  let expiryDate = $derived.by(() => {
    if (expirySelection === 'none') return null;

    let now = new Date();

    if (expirySelection === 'custom') {
      if (!expiryDateInput) return null;

      let timeSuffix = now.toISOString().slice(10);
      return new Date(expiryDateInput + timeSuffix);
    }

    return new Date(
      now.getFullYear(),
      now.getMonth(),
      now.getDate() + Number(expirySelection),
      now.getHours(),
      now.getMinutes(),
      now.getSeconds(),
    );
  });

  let expiryDescription = $derived(
    expirySelection === 'none'
      ? 'The token will never expire'
      : `The token will expire on ${expiryDate?.toLocaleDateString(undefined, { dateStyle: 'long' })}`,
  );

  function toggleScope(scope: string): void {
    scopes = scopes.includes(scope) ? scopes.filter(it => it !== scope) : [...scopes, scope];
    scopesInvalid = false;
  }

  function updateExpirySelection(event: Event): void {
    expiryDateInput = expiryDate?.toISOString().slice(0, 10) ?? '';
    expirySelection = (event.target as HTMLSelectElement).value;
  }

  function addCratePattern(): void {
    crateScopes = [...crateScopes, new CratePattern('')];
  }

  function removeCrateScope(index: number): void {
    crateScopes = crateScopes.filter((_, i) => i !== index);
  }

  function validate(): boolean {
    nameInvalid = !name;
    expiryDateInvalid = expirySelection === 'custom' && !expiryDateInput;
    scopesInvalid = scopes.length === 0;
    let crateScopesValid = crateScopes
      .map(pattern => {
        let valid = isValidPattern(pattern.pattern);
        pattern.showAsInvalid = !valid;
        return valid;
      })
      .every(Boolean);

    return !nameInvalid && !expiryDateInvalid && !scopesInvalid && crateScopesValid;
  }

  async function handleSubmit(event: SubmitEvent): Promise<void> {
    event.preventDefault();

    if (!validate()) return;

    isSaving = true;

    let crateScopePatterns: string[] | null = crateScopes.map(it => it.pattern);
    if (crateScopePatterns.length === 0) {
      crateScopePatterns = null;
    }

    try {
      let result = await client.PUT('/api/v1/me/tokens', {
        body: {
          api_token: {
            name,
            endpoint_scopes: scopes,
            crate_scopes: crateScopePatterns,
            expired_at: expiryDate?.toISOString() ?? null,
          },
        },
      });

      if (result.error) {
        throw new Error();
      }

      let apiToken = result.data.api_token;
      tokenPageState.pendingToken = { id: apiToken.id, token: apiToken.token };

      await goto(resolve('/settings/tokens'));
    } catch {
      notifications.error('An error has occurred while generating your API token. Please try again later!');
    } finally {
      isSaving = false;
    }
  }
</script>

<svelte:head>
  <title>New API Token | crates.io: Rust Package Registry</title>
</svelte:head>

<PageHeader title="Account Settings" />

<SettingsPage>
  <h2>New API Token</h2>

  <!-- TODO: Pre-fill form from existing token (Phase 6) -->

  <form class="form" onsubmit={handleSubmit}>
    <div class="form-group" data-test-name-group>
      <label for="{id}-name" class="form-group-name">Name</label>

      <!-- svelte-ignore a11y_autofocus -->
      <input
        id="{id}-name"
        type="text"
        bind:value={name}
        disabled={isSaving}
        autocomplete="off"
        aria-required="true"
        aria-invalid={nameInvalid}
        class="name-input base-input"
        data-test-name
        autofocus
        oninput={() => (nameInvalid = false)}
      />

      {#if nameInvalid}
        <div class="form-group-error" data-test-error>Please enter a name for this token.</div>
      {/if}
    </div>

    <div class="form-group" data-test-expiry-group>
      <label for="{id}-expiry" class="form-group-name">Expiration</label>

      <div class="select-group">
        <select
          id="{id}-expiry"
          disabled={isSaving}
          class="expiry-select base-input"
          data-test-expiry
          onchange={updateExpirySelection}
        >
          <option value="none">No expiration</option>
          <option value="7">7 days</option>
          <option value="30">30 days</option>
          <option value="60">60 days</option>
          <option value="90" selected>90 days</option>
          <option value="365">365 days</option>
          <option value="custom">Custom...</option>
        </select>

        {#if expirySelection === 'custom'}
          <input
            type="date"
            bind:value={expiryDateInput}
            min={today}
            disabled={isSaving}
            aria-invalid={expiryDateInvalid}
            aria-label="Custom expiration date"
            class="expiry-date-input base-input"
            data-test-expiry-date
            oninput={() => (expiryDateInvalid = false)}
          />
        {:else}
          <span class="expiry-description" data-test-expiry-description>
            {expiryDescription}
          </span>
        {/if}
      </div>
    </div>

    <div class="form-group" data-test-scopes-group>
      <div class="form-group-name">
        Scopes

        <a
          href="https://rust-lang.github.io/rfcs/2947-crates-io-token-scopes.html"
          target="_blank"
          rel="noopener noreferrer"
          class="help-link"
        >
          <span class="sr-only">Help</span>
          <CircleQuestionIcon />
        </a>
      </div>

      <ul role="list" class="scopes-list" class:invalid={scopesInvalid}>
        {#each ENDPOINT_SCOPES as scope (scope)}
          <li>
            <label data-test-scope={scope}>
              <input
                type="checkbox"
                checked={scopes.includes(scope)}
                disabled={isSaving}
                onchange={() => toggleScope(scope)}
              />

              <span class="scope-id">{scope}</span>
              <span class="scope-description">{scopeDescription(scope)}</span>
            </label>
          </li>
        {/each}
      </ul>

      {#if scopesInvalid}
        <div class="form-group-error" data-test-error>Please select at least one token scope.</div>
      {/if}
    </div>

    <div class="form-group" data-test-scopes-group>
      <div class="form-group-name">
        Crates

        <a
          href="https://rust-lang.github.io/rfcs/2947-crates-io-token-scopes.html"
          target="_blank"
          rel="noopener noreferrer"
          class="help-link"
        >
          <span class="sr-only">Help</span>
          <CircleQuestionIcon />
        </a>
      </div>

      <ul role="list" class="crates-list">
        {#each crateScopes as pattern, index (pattern)}
          <li class="crates-scope" class:invalid={pattern.showAsInvalid} data-test-crate-pattern={index}>
            <div>
              <input
                bind:value={pattern.pattern}
                aria-label="Crate name pattern"
                oninput={() => (pattern.showAsInvalid = false)}
                onblur={() => {
                  let valid = pattern.isValid || pattern.pattern === '';
                  pattern.showAsInvalid = !valid;
                }}
              />

              <span class="pattern-description" data-test-description>
                {#if !pattern.pattern}
                  Please enter a crate name pattern
                {:else if pattern.isValid}
                  <PatternDescription pattern={pattern.pattern} />
                {:else}
                  Invalid crate name pattern
                {/if}
              </span>
            </div>

            <button type="button" data-test-remove onclick={() => removeCrateScope(index)}>
              <span class="sr-only">Remove pattern</span>
              <TrashIcon />
            </button>
          </li>
        {:else}
          <li class="crates-unrestricted" data-test-crates-unrestricted>
            <strong>Unrestricted</strong>
            – This token can be used for all of your crates.
          </li>
        {/each}

        <li class="crates-pattern-button">
          <button type="button" data-test-add-crate-pattern onclick={addCratePattern}> Add pattern </button>
        </li>
      </ul>
    </div>

    <div class="buttons">
      <button type="submit" class="generate-button button button--small" disabled={isSaving} data-test-generate>
        Generate Token

        {#if isSaving}
          <LoadingSpinner theme="light" class="spinner" data-test-spinner />
        {/if}
      </button>

      <a href={resolve('/settings/tokens')} class="cancel-button button button--tan button--small" data-test-cancel>
        Cancel
      </a>
    </div>
  </form>
</SettingsPage>

<style>
  .form-group,
  .buttons {
    position: relative;
    margin: var(--space-m) 0;
  }

  .select-group {
    display: flex;
    align-content: center;
    align-items: center;
  }

  .help-link {
    flex-shrink: 0;
    color: light-dark(var(--grey600), var(--grey700));
    padding: var(--space-3xs);
    margin: calc(-1 * var(--space-3xs));

    &:hover {
      color: light-dark(var(--grey700), var(--grey600));
    }

    :global(svg) {
      width: 1em;
      height: 1em;
    }
  }

  .buttons {
    display: flex;
    gap: var(--space-2xs);
    flex-wrap: wrap;
  }

  .name-input {
    max-width: 440px;
    width: 100%;
  }

  .expiry-select {
    padding-right: var(--space-m);
    background-image: url('$lib/assets/dropdown-black.svg');
    background-repeat: no-repeat;
    background-position: calc(100% - var(--space-2xs)) center;
    background-size: 10px;
    appearance: none;

    :global([data-color-scheme='system']) & {
      @media (prefers-color-scheme: dark) {
        background-image: url('$lib/assets/dropdown-white.svg');
      }
    }

    :global([data-color-scheme='dark']) & {
      background-image: url('$lib/assets/dropdown-white.svg');
    }
  }

  .expiry-date-input {
    margin-left: var(--space-2xs);
  }

  .expiry-description {
    margin-left: var(--space-2xs);
    font-size: 0.9em;
  }

  .scopes-list {
    list-style: none;
    padding: 0;
    margin: 0;
    background-color: light-dark(white, #141413);
    border: 1px solid var(--gray-border);
    border-radius: var(--space-3xs);

    &.invalid {
      background: light-dark(#fff2f2, #170808);
      border-color: red;
    }

    > :global(* + *) {
      border-top: inherit;
    }

    label {
      padding: var(--space-xs) var(--space-s);
      display: flex;
      flex-wrap: wrap;
      gap: var(--space-xs);
      font-size: 0.9em;
    }
  }

  .scope-id {
    display: inline-block;
    max-width: 170px;
    flex-grow: 1;
    font-weight: bold;
  }

  .scope-description {
    display: inline-block;
  }

  .crates-list {
    list-style: none;
    padding: 0;
    margin: 0;
    background-color: light-dark(white, #141413);
    border: 1px solid var(--gray-border);
    border-radius: var(--space-3xs);

    > :global(* + *) {
      border-top: inherit;
    }
  }

  .crates-unrestricted {
    padding: var(--space-xs) var(--space-s);
    font-size: 0.9em;
  }

  .crates-scope {
    display: flex;

    > div {
      padding: var(--space-xs) var(--space-s);
      display: flex;
      flex-wrap: wrap;
      gap: var(--space-xs);
      font-size: 0.9em;
      flex-grow: 1;
    }

    input {
      margin: calc(-1 * var(--space-4xs)) 0;
      padding: var(--space-3xs) var(--space-2xs);
      border: 1px solid var(--gray-border);
      border-radius: var(--space-3xs);
    }

    &.invalid input {
      background: light-dark(#fff2f2, #170808);
      border-color: red;
    }

    > button {
      margin: 0;
      padding: 0 var(--space-xs);
      border: none;
      background: none;
      cursor: pointer;
      color: var(--grey700);
      flex-shrink: 0;
      display: flex;
      align-items: center;

      &:hover {
        background: light-dark(var(--grey200), #333333);
        color: light-dark(var(--grey900), white);
      }

      :global(svg) {
        height: 1.1em;
        width: 1.1em;
      }
    }

    &:first-child button {
      border-top-right-radius: var(--space-3xs);
    }
  }

  .pattern-description {
    flex-grow: 1;
    align-self: center;

    .invalid & {
      color: red;
    }
  }

  .crates-pattern-button button {
    padding: var(--space-xs) var(--space-s);
    font-size: 0.9em;
    width: 100%;
    border: none;
    background: none;
    border-bottom-left-radius: var(--space-3xs);
    border-bottom-right-radius: var(--space-3xs);
    cursor: pointer;
    font-weight: bold;

    &:hover {
      background: light-dark(var(--grey200), #333333);
    }
  }

  .generate-button {
    border-radius: 4px;

    :global(.spinner) {
      margin-left: var(--space-2xs);
    }
  }

  .cancel-button {
    border-radius: 4px;
  }
</style>
