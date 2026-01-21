<script lang="ts">
  import type { components } from '@crates-io/api-client';

  import { resolve } from '$app/paths';

  import logo from '$lib/assets/cargo.png';
  import LockIcon from '$lib/assets/lock.svg?component';
  import ColorSchemeMenu from './ColorSchemeMenu.svelte';
  import * as Dropdown from './dropdown';
  import LoadingSpinner from './LoadingSpinner.svelte';
  import SearchForm from './SearchForm.svelte';
  import UserAvatar from './UserAvatar.svelte';

  type AuthenticatedUser = components['schemas']['AuthenticatedUser'];

  interface Props {
    hero?: boolean;
    currentUser?: AuthenticatedUser | null;
  }

  let { hero = false, currentUser }: Props = $props();

  let isAdmin = $derived(currentUser?.is_admin ?? false);

  // TODO: implement session state
  let isLoggingIn = $state(false);
  let isLoggingOut = $state(false);

  function login() {
    isLoggingIn = true;
  }

  function logout() {
    isLoggingOut = true;
  }

  // TODO: implement sudo actions
  const SUDO_SESSION_DURATION_MS = 6 * 60 * 60 * 1000;

  let isSudoEnabled = $state(false);
  let sudoEnabledUntil = $state<Date | null>(null);

  function enableSudo() {
    isSudoEnabled = true;
    sudoEnabledUntil = new Date(Date.now() + SUDO_SESSION_DURATION_MS);
  }

  function disableSudo() {
    isSudoEnabled = false;
    sudoEnabledUntil = null;
  }

  function formatTime(date: Date | null): string {
    if (!date) return '';
    return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }
</script>

<header class="header" class:hero>
  <div class="header-inner width-limit">
    <a href={resolve('/')} class="index-link">
      <img src={logo} role="none" alt="" class="logo" />
      <h1>crates.io</h1>
    </a>

    <div class="search-form">
      <h1 class="hero-title">The Rust community's crate registry</h1>

      <SearchForm size={hero ? 'big' : undefined} autofocus={hero} />
    </div>

    <nav class="nav">
      <ColorSchemeMenu class="color-scheme-menu" />

      <a href={resolve('/crates')} data-test-all-crates-link> Browse All Crates </a>
      <span class="sep">|</span>

      {#if currentUser}
        <Dropdown.Root data-test-user-menu>
          <Dropdown.Trigger class="button-reset" data-test-toggle>
            {#if isSudoEnabled}
              <span class="wizard-hat" data-test-wizard-hat>🧙</span>
            {/if}

            <UserAvatar
              user={{ ...currentUser, kind: 'user' }}
              size="small"
              style="margin-right: var(--space-2xs); margin-top: calc((22px - 1em) * -0.5)"
              data-test-avatar
            />

            {currentUser.name}
          </Dropdown.Trigger>

          <Dropdown.Menu class="current-user-links">
            <Dropdown.Item>
              <a href={resolve('/users/[user_id]', { user_id: currentUser.login })}>Profile</a>
            </Dropdown.Item>
            <Dropdown.Item><a href={resolve('/dashboard')}>Dashboard</a></Dropdown.Item>
            <Dropdown.Item><a href={resolve('/settings')} data-test-settings>Account Settings</a></Dropdown.Item>
            <Dropdown.Item><a href={resolve('/me/pending-invites')}>Owner Invites</a></Dropdown.Item>
            {#if isAdmin}
              <Dropdown.Item class="sudo">
                {#if isSudoEnabled}
                  <button
                    type="button"
                    class="sudo-menu-item button-reset"
                    data-test-disable-admin-actions
                    onclick={disableSudo}
                  >
                    Disable admin actions
                    <span class="expires-in">expires at {formatTime(sudoEnabledUntil)}</span>
                  </button>
                {:else}
                  <button
                    type="button"
                    class="sudo-menu-item button-reset"
                    data-test-enable-admin-actions
                    onclick={enableSudo}
                  >
                    Enable admin actions
                  </button>
                {/if}
              </Dropdown.Item>
            {/if}
            <Dropdown.Item style="border-top: 1px solid var(--gray-border)">
              <button
                type="button"
                class="logout-menu-item button-reset"
                disabled={isLoggingOut}
                data-test-logout-button
                onclick={logout}
              >
                {#if isLoggingOut}
                  <LoadingSpinner class="spinner" />
                {/if}
                Sign Out
              </button>
            </Dropdown.Item>
          </Dropdown.Menu>
        </Dropdown.Root>
      {:else}
        <button
          type="button"
          class="login-button button-reset"
          disabled={isLoggingIn}
          data-test-login-button
          onclick={login}
        >
          {#if isLoggingIn}
            <LoadingSpinner class="spinner" />
          {:else}
            <LockIcon />
          {/if}
          Log in with GitHub
        </button>
      {/if}
    </nav>

    <div class="menu">
      <ColorSchemeMenu class="color-scheme-menu" />

      <Dropdown.Root>
        <Dropdown.Trigger class="button-reset">Menu</Dropdown.Trigger>
        <Dropdown.Menu class="current-user-links">
          <Dropdown.Item><a href={resolve('/crates')}>Browse All Crates</a></Dropdown.Item>
          {#if currentUser}
            <Dropdown.Item>
              <a href={resolve('/users/[user_id]', { user_id: currentUser.login })}>Profile</a>
            </Dropdown.Item>
            <Dropdown.Item><a href={resolve('/dashboard')}>Dashboard</a></Dropdown.Item>
            <Dropdown.Item><a href={resolve('/settings')} data-test-me-link>Account Settings</a></Dropdown.Item>
            <Dropdown.Item><a href={resolve('/me/pending-invites')}>Owner Invites</a></Dropdown.Item>
            <Dropdown.Item style="border-top: 1px solid var(--gray-border)">
              <button type="button" class="logout-menu-item button-reset" disabled={isLoggingOut} onclick={logout}>
                {#if isLoggingOut}
                  <LoadingSpinner class="spinner" />
                {/if}
                Sign Out
              </button>
            </Dropdown.Item>
          {:else}
            <Dropdown.Item>
              <button type="button" class="login-menu-item button-reset" disabled={isLoggingIn} onclick={login}>
                {#if isLoggingIn}
                  <LoadingSpinner class="spinner" />
                {/if}
                Log in with GitHub
              </button>
            </Dropdown.Item>
          {/if}
        </Dropdown.Menu>
      </Dropdown.Root>
    </div>
  </div>
</header>

<style>
  .header {
    display: flex;
    justify-content: center;
    width: 100%;
  }

  .header-inner {
    display: grid;
    grid-template:
      'logo search nav' auto /
      auto 1fr auto;
    align-items: center;
    padding: var(--space-xs) var(--space-m);
    color: white;

    & a {
      color: white;
      text-decoration: none;

      &:hover {
        color: white;
      }
    }

    @media only screen and (max-width: 900px) {
      grid-template:
        'logo search menu' auto /
        auto 1fr auto;
    }

    @media only screen and (max-width: 820px) {
      grid-template:
        'logo menu' auto
        'search search' auto /
        auto 1fr;
    }

    .hero & {
      grid-template:
        'logo nav' auto
        'search search' auto /
        auto 1fr;

      @media only screen and (max-width: 900px) {
        grid-template:
          'logo menu' auto
          'search search' auto /
          auto 1fr;
      }
    }
  }

  .index-link {
    grid-area: logo;
    display: flex;
    align-items: center;

    & h1 {
      margin: 0;
      font-size: var(--space-m);
    }
  }

  .logo {
    width: auto;
    height: calc(var(--space-m) * 1.4);
    margin-right: var(--space-xs);
  }

  .search-form {
    grid-area: search;
    margin: 0 var(--space-m);

    @media only screen and (max-width: 820px) {
      margin: var(--space-s) 0;
    }

    .hero & {
      justify-self: center;
      padding: var(--space-l) 0 var(--space-l-xl);
      margin: 0;
    }
  }

  .hero-title {
    display: none;
    margin: 0 0 var(--space-m);
    font-size: var(--space-m-l);
    text-align: center;
    color: white;
    text-shadow: 1px 3px 2px var(--green900);

    .hero & {
      display: block;
    }
  }

  .sep {
    margin: 0 var(--space-2xs);
    opacity: 0.5;
  }

  .nav {
    grid-area: nav;
    display: flex;
    align-items: center;
    justify-self: end;

    @media only screen and (max-width: 900px) {
      display: none;
    }
  }

  .menu {
    grid-area: menu;
    justify-self: end;
    display: none;

    @media only screen and (max-width: 900px) {
      display: flex;
      align-items: center;
    }
  }

  .header :global(.color-scheme-menu) {
    margin-right: var(--space-xs);
  }

  .login-button {
    display: inline-flex;
    align-items: center;
    /* negative margin for larger click target */
    margin: calc(var(--space-2xs) * -1);
    padding: var(--space-2xs);
    cursor: pointer;

    &:disabled {
      cursor: wait;
    }

    & :global(.spinner) {
      --spinner-color: white;
      --spinner-bg-color: rgba(255, 255, 255, 0.2);

      margin-right: var(--space-2xs);
    }
  }

  .login-button :global(svg) {
    width: 1em;
    margin-right: var(--space-2xs);
    opacity: 0.5;
  }

  .wizard-hat {
    margin-right: var(--space-3xs);
  }

  .menu,
  .nav {
    & :global(.current-user-links) {
      left: auto;
      right: 0;
      min-width: 200px;
    }
  }

  .login-menu-item,
  .logout-menu-item,
  .sudo-menu-item {
    cursor: pointer;

    &:disabled {
      cursor: wait;
    }

    & :global(.spinner) {
      margin-right: var(--space-2xs);
    }
  }

  .sudo-menu-item {
    flex-direction: column;

    > .expires-in {
      font-size: 80%;
      font-style: italic;
      padding-top: var(--space-3xs);
    }
  }
</style>
