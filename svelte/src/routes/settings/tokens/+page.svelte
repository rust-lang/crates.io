<script lang="ts">
  import type { components } from '@crates-io/api-client';

  import { onDestroy } from 'svelte';
  import { browser } from '$app/environment';
  import { resolve } from '$app/paths';
  import { createClient } from '@crates-io/api-client';
  import { formatDistanceToNow } from 'date-fns';
  import { SvelteSet } from 'svelte/reactivity';

  import CopyIcon from '$lib/assets/copy.svg?component';
  import CopyButton from '$lib/components/CopyButton.svelte';
  import LoadingSpinner from '$lib/components/LoadingSpinner.svelte';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import PageTitle from '$lib/components/PageTitle.svelte';
  import PatternDescription from '$lib/components/PatternDescription.svelte';
  import SettingsPage from '$lib/components/SettingsPage.svelte';
  import Tooltip from '$lib/components/Tooltip.svelte';
  import { getNotifications } from '$lib/notifications.svelte';
  import { scopeDescription } from '$lib/utils/token-scopes';
  import { getTokenPageState } from './+layout.svelte';

  type ApiToken = components['schemas']['ApiToken'];

  let notifications = getNotifications();
  let client = createClient({ fetch });

  let { data } = $props();

  let revokedTokenIds = new SvelteSet<number>();
  let revokingTokenIds = new SvelteSet<number>();

  let tokenPageState = getTokenPageState();
  let pendingToken = $derived(tokenPageState.pendingToken);

  let isClipboardSupported = browser && Boolean(navigator.clipboard?.writeText);

  onDestroy(() => {
    tokenPageState.pendingToken = null;
  });

  let sortedTokens = $derived(sortTokens(data.tokens.filter(t => !revokedTokenIds.has(t.id))));

  function isExpired(token: ApiToken): boolean {
    return !!token.expired_at && new Date(token.expired_at) < new Date();
  }

  function sortTokens(tokens: ApiToken[]): ApiToken[] {
    return [...tokens].sort((a, b) => {
      let aExpired = isExpired(a);
      let bExpired = isExpired(b);

      if (aExpired && !bExpired) return 1;
      if (bExpired && !aExpired) return -1;

      return a.created_at < b.created_at ? 1 : -1;
    });
  }

  function formatScopes(scopes: string[]): { type: string; value: string }[] {
    // We hardcode `en-US` here because the rest of the interface text is also currently displayed only in English.
    return new Intl.ListFormat('en-US').formatToParts(scopes);
  }

  async function revokeToken(token: ApiToken) {
    revokingTokenIds.add(token.id);
    try {
      let result = await client.DELETE('/api/v1/me/tokens/{id}', {
        params: { path: { id: token.id } },
      });

      if (result.error) {
        throw new Error();
      }

      revokedTokenIds.add(token.id);
    } catch {
      notifications.error('An unknown error occurred while revoking this token');
    } finally {
      revokingTokenIds.delete(token.id);
    }
  }
</script>

<PageTitle title="Settings" />

<PageHeader title="Account Settings" />

<SettingsPage>
  <div class="me-subheading">
    <h2>API Tokens</h2>
    <div class="right">
      <a href={resolve('/settings/tokens/new')} class="button" data-test-new-token-button>New Token</a>
    </div>
  </div>

  <p class="explainer">
    You can use the API tokens generated on this page to run
    <a href="https://doc.rust-lang.org/cargo/">cargo</a>
    commands that need write access to crates.io. If you want to publish your own crates then this is required.
  </p>

  <p class="explainer">
    To prevent keys being silently leaked they are stored on crates.io in hashed form. This means you can only download
    keys when you first create them. If you have old unused keys you can safely delete them and create a new one.
  </p>

  <p class="explainer">
    To use an API token, run
    <a href="https://doc.rust-lang.org/cargo/commands/cargo-login.html"><code>cargo login</code></a>
    on the command line and paste the key when prompted. This will save it to a
    <a href="https://doc.rust-lang.org/cargo/reference/config.html#credentials">local credentials file</a>. For CI
    systems you can use the
    <a href="https://doc.rust-lang.org/cargo/reference/config.html?highlight=CARGO_REGISTRY_TOKEN#credentials"
      ><code>CARGO_REGISTRY_TOKEN</code></a
    >
    environment variable, but make sure that the token stays secret!
  </p>

  {#if sortedTokens.length > 0}
    <ul role="list" class="token-list">
      {#each sortedTokens as token (token.id)}
        <li class="row" class:expired={isExpired(token)} data-test-api-token={token.id}>
          <h3 class="name" data-test-name>
            {token.name}
          </h3>

          {#if token.endpoint_scopes || token.crate_scopes}
            <div class="scopes text--small">
              {#if token.endpoint_scopes}
                <div class="endpoint-scopes" data-test-endpoint-scopes>
                  Scopes:
                  {#each formatScopes(token.endpoint_scopes) as part, i (i)}{#if part.type === 'element'}<strong
                        >{part.value}<Tooltip text={scopeDescription(part.value)} /></strong
                      >{:else}{part.value}{/if}{/each}
                </div>
              {/if}

              {#if token.crate_scopes}
                <div class="crate-scopes" data-test-crate-scopes>
                  Crates:
                  {#each formatScopes(token.crate_scopes) as part, i (i)}{#if part.type === 'element'}<strong
                        >{part.value}<Tooltip>
                          <PatternDescription pattern={part.value} />
                        </Tooltip></strong
                      >{:else}{part.value}{/if}{/each}
                </div>
              {/if}
            </div>
          {/if}

          <div class="metadata text--small">
            <div title={token.last_used_at ?? undefined} class="last-used-at" data-test-last-used-at>
              {#if token.last_used_at}
                Last used {formatDistanceToNow(token.last_used_at, { addSuffix: true })}
              {:else}
                Never used
              {/if}
            </div>

            <div title={token.created_at} class="created-at" data-test-created-at>
              Created {formatDistanceToNow(token.created_at, { addSuffix: true })}
            </div>

            {#if token.expired_at}
              <div title={token.expired_at} class="expired-at" data-test-expired-at>
                {isExpired(token) ? 'Expired' : 'Expires'}
                {formatDistanceToNow(token.expired_at, { addSuffix: true })}
              </div>
            {/if}
          </div>

          {#if pendingToken?.id === token.id}
            <div class="new-token">
              <div class="new-token-explainer">
                Make sure to copy your API token now. You won't be able to see it again!
              </div>

              <div class="token-display">
                <span class="token-value" data-test-token>{pendingToken.token}</span>

                {#if isClipboardSupported}
                  <CopyButton copyText={pendingToken.token} class="copy-button button-reset">
                    <span class="sr-only">Copy</span>
                    <CopyIcon aria-hidden="true" class="copy-button-icon" />
                  </CopyButton>
                {/if}
              </div>
            </div>
          {/if}

          <div class="actions">
            <!-- eslint-disable svelte/no-navigation-without-resolve -->
            <a
              href={`${resolve('/settings/tokens/new')}?from=${token.id}`}
              class="regenerate-button button button--small"
              data-test-regenerate-token-button
            >
              Regenerate
            </a>
            <!-- eslint-enable svelte/no-navigation-without-resolve -->
            {#if !isExpired(token)}
              <button
                type="button"
                class="revoke-button button button--tan button--small"
                disabled={revokingTokenIds.has(token.id)}
                data-test-revoke-token-button
                onclick={() => revokeToken(token)}
              >
                Revoke
              </button>
              {#if revokingTokenIds.has(token.id)}
                <LoadingSpinner class="spinner" data-test-saving-spinner />
              {/if}
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  {:else}
    <div class="empty-state">
      <div class="empty-state-label">You have not generated any API tokens yet.</div>

      <a
        href={resolve('/settings/tokens/new')}
        class="empty-state-button button button--small"
        data-test-empty-state-button
      >
        New Token
      </a>
    </div>
  {/if}
</SettingsPage>

<style>
  .me-subheading {
    display: flex;

    .right {
      flex: 2;
      display: flex;
      justify-content: flex-end;
      align-self: center;
    }
  }

  .explainer {
    line-height: 1.5;
  }

  .token-list {
    margin: var(--space-m) 0;
    padding: 0;
    list-style: none;
    border-radius: var(--space-3xs);
    background-color: light-dark(white, #141413);
    box-shadow: 0 1px 3px light-dark(hsla(51, 90%, 42%, 0.35), #232321);

    > * {
      padding: var(--space-m);
    }

    > * + * {
      border-top: 1px solid light-dark(hsla(51, 90%, 42%, 0.25), #424242);
    }
  }

  .name {
    margin: 0 0 var(--space-s);
    font-weight: 500;
  }

  .scopes,
  .metadata {
    > * + * {
      margin-top: var(--space-3xs);
    }
  }

  .scopes {
    margin-bottom: var(--space-xs);
  }

  .actions {
    margin-top: var(--space-s);
    display: flex;
    align-items: center;
  }

  .regenerate-button {
    flex-grow: 1;
    border-radius: var(--space-3xs);
  }

  .revoke-button {
    flex-grow: 1;
    border-radius: var(--space-3xs);
  }

  .actions > :global(.spinner) {
    margin-left: var(--space-xs);
  }

  .new-token {
    margin-top: var(--space-s);
  }

  .new-token-explainer {
    font-size: 20px;
  }

  .token-display {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    background: var(--main-color);
    color: light-dark(white, #141413);
    font-family: var(--font-monospace);
    border-radius: var(--space-3xs);
    margin-top: var(--space-xs);
  }

  .token-value {
    padding: var(--space-s);
    user-select: all;
  }

  .token-display :global(.copy-button) {
    align-self: stretch;
    padding: 0 var(--space-s);
    cursor: pointer;

    &:hover {
      color: light-dark(#ddd8b2, #65655e);
    }
  }

  .token-display :global(.copy-button-icon) {
    width: auto;
    height: 1.3em;
  }

  .empty-state {
    display: grid;
    place-items: center;
    align-content: center;
    margin: var(--space-m) 0;
    padding: var(--space-xl-2xl);
    border: 2px light-dark(black, white) dashed;
    border-radius: var(--space-3xs);
    background-color: light-dark(white, #141413);
    box-shadow: 0 2px 3px light-dark(hsla(51, 50%, 45%, 0.35), #232321);
  }

  .empty-state-label {
    font-size: 20px;
  }

  .empty-state-button {
    margin-top: var(--space-m);
    border-radius: 4px;
  }

  .expired {
    opacity: 0.6;
  }

  @media (min-width: 640px) {
    .row {
      display: grid;
      grid-template:
        'name actions' auto
        'scopes actions' auto
        'metadata actions' auto
        'details details' auto
        / 1fr auto;

      .scopes {
        grid-area: scopes;
      }

      .metadata {
        grid-area: metadata;
      }

      .new-token {
        grid-area: details;
        margin-bottom: 0;
      }

      .actions {
        display: flex;
        flex-direction: column;
        grid-area: actions;
        align-self: start;
        margin: 0 0 0 var(--space-xs);
      }

      .actions > :global(*) {
        flex-grow: 1;
        width: 100%;

        & + :global(*) {
          margin-top: var(--space-xs);
        }
      }
    }
  }
</style>
