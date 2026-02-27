<script lang="ts">
  import type { components } from '@crates-io/api-client';

  import { resolve } from '$app/paths';
  import { formatDistanceToNow } from 'date-fns';

  import PageHeader from '$lib/components/PageHeader.svelte';
  import PatternDescription from '$lib/components/PatternDescription.svelte';
  import SettingsPage from '$lib/components/SettingsPage.svelte';
  import Tooltip from '$lib/components/Tooltip.svelte';
  import { scopeDescription } from '$lib/utils/token-scopes';

  type ApiToken = components['schemas']['ApiToken'];

  let { data } = $props();

  let sortedTokens = $derived(sortTokens(data.tokens));

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
</script>

<svelte:head>
  <title>API Tokens | crates.io: Rust Package Registry</title>
</svelte:head>

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

          <!-- TODO: Raw token display (Phase 5) -->

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
            <!-- TODO: Revoke button (Phase 3) -->
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
