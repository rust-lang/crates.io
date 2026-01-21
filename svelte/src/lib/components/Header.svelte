<script lang="ts">
  import type { components } from '@crates-io/api-client';

  import { resolve } from '$app/paths';

  import logo from '$lib/assets/cargo.png';
  import LockIcon from '$lib/assets/lock.svg?component';
  import ColorSchemeMenu from './ColorSchemeMenu.svelte';
  import SearchForm from './SearchForm.svelte';

  // TODO: import Dropdown from './Dropdown.svelte';
  // TODO: import LoadingSpinner from './LoadingSpinner.svelte';
  // TODO: import UserAvatar from './UserAvatar.svelte';

  type AuthenticatedUser = components['schemas']['AuthenticatedUser'];

  interface Props {
    hero?: boolean;
    currentUser?: AuthenticatedUser | null;
  }

  let { hero = false, currentUser }: Props = $props();

  // TODO: implement session state
  // let isAdmin = $derived(currentUser?.is_admin ?? false);
  // let isSudoEnabled = $derived(session.isSudoEnabled);
  // let sudoEnabledUntil = $derived(session.sudoEnabledUntil);

  // TODO: implement sudo actions
  // const SUDO_SESSION_DURATION_MS = 6 * 60 * 60 * 1000;
  // function enableSudo() { session.setSudo(SUDO_SESSION_DURATION_MS); }
  // function disableSudo() { session.setSudo(0); }
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
        {currentUser.name}
        <!-- TODO: implement user menu -->
        <!--   <Dropdown data-test-user-menu> ... </Dropdown> -->
      {:else}
        <button type="button" class="login-button button-reset" data-test-login-button>
          <LockIcon />
          Log in with GitHub
        </button>
      {/if}
    </nav>

    <div class="menu">
      <ColorSchemeMenu class="color-scheme-menu" />

      <!-- TODO: implement mobile menu dropdown -->
      <!-- <Dropdown>
        <button class="dropdown-button">Menu</button>
        <menu>
          <li><a href={resolve('/crates')}>Browse All Crates</a></li>
          {#if currentUser}
            <li><a href={resolve(`/users/${currentUser.login}`)}>Profile</a></li>
            <li><a href={resolve('/dashboard')}>Dashboard</a></li>
            <li><a href={resolve('/settings')}>Account Settings</a></li>
            <li><a href={resolve('/me/pending-invites')}>Owner Invites</a></li>
            <li><button>Sign Out</button></li>
          {:else}
            <li><button>Log in with GitHub</button></li>
          {/if}
        </menu>
      </Dropdown> -->
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

  /* TODO: uncomment when menu is added
  .menu-item-with-separator {
    border-top: 1px solid var(--gray-border);
  }
  */

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

    /* TODO: uncomment when SVG icons are added
    & .spinner {
      --spinner-color: white;
      --spinner-bg-color: rgba(255, 255, 255, .2);

      margin-right: var(--space-2xs);
    }
    */
  }

  .login-button :global(svg) {
    width: 1em;
    margin-right: var(--space-2xs);
    opacity: 0.5;
  }

  /* TODO: uncomment when UserAvatar is implemented
  .avatar {
    margin-right: var(--space-2xs);
  }

  .wizard-hat {
    margin-right: var(--space-3xs);
  }
  */

  /* TODO: uncomment when Dropdown is implemented
  .current-user-links {
    left: auto;
    right: 0;
    min-width: 200px;
  }

  .dropdown-button {
    background: none;
    border: 0;
    padding: 0;

    & img {
      margin-top: calc((22px - 1em) * -0.5);
    }
  }

  .menu-item-with-separator {
    border-top: 1px solid var(--gray-border);
  }

  .login-menu-item,
  .logout-menu-item,
  .sudo-menu-item {
    cursor: pointer;

    &:disabled {
      cursor: wait;
    }

    & .spinner {
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
  */
</style>
