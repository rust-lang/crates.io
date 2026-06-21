<script lang="ts">
  import { afterNavigate, goto } from '$app/navigation';
  import { resolve } from '$app/paths';

  import Icon from '$lib/components/Icon.svelte';
  import { getSearchFormContext } from '$lib/search-form.svelte';

  interface Props {
    size?: 'big';
    autofocus?: boolean;
  }

  let { size, autofocus = false }: Props = $props();

  let searchFormContext = getSearchFormContext();
  let inputElement: HTMLInputElement | undefined = $state();

  // Focus the input imperatively instead of using the `autofocus` attribute.
  // Svelte only sets the attribute on mount and never removes it, and
  // SvelteKit's `reset_focus()` refocuses any `[autofocus]` element after
  // each client-side navigation, which would steal focus back to the
  // search bar on every nav once the attribute has been set.
  let hasAutoFocused = false;
  afterNavigate(() => {
    if (autofocus && !hasAutoFocused) {
      hasAutoFocused = true;
      inputElement?.focus();
    }
  });

  function search(event: SubmitEvent) {
    event.preventDefault();
    // eslint-disable-next-line svelte/no-navigation-without-resolve -- resolve() doesn't support query params
    goto(`${resolve('/search')}?q=${encodeURIComponent(searchFormContext.value)}`, { keepFocus: true });
  }

  function handleKeydown(event: KeyboardEvent) {
    // Don't trigger if user is typing in an input/textarea or if modifier keys are pressed.
    // `event.target` is retargeted to the shadow host for events originating
    // inside a shadow root (e.g. an input within a web component), so read the
    // real innermost element from the composed path instead.
    let target = event.composedPath()[0] as HTMLElement;
    if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) {
      return;
    }
    if (event.ctrlKey || event.altKey || event.metaKey) {
      return;
    }

    if (['s', 'S', '/'].includes(event.key)) {
      event.preventDefault();
      inputElement?.focus();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<form
  action="/search"
  role="search"
  autocapitalize="off"
  autocomplete="off"
  spellcheck="false"
  data-test-search-form
  class="form"
  class:size-big={size === 'big'}
  onsubmit={search}
>
  <!-- Large screen input with keyboard shortcut hint -->
  <input
    bind:this={inputElement}
    type="text"
    inputmode="search"
    class="input-lg"
    name="q"
    placeholder="Type 'S' or '/' to search"
    autocorrect="off"
    bind:value={searchFormContext.value}
    required
    aria-label="Search"
    data-test-search-input
  />

  <!-- Small screen input with shorter placeholder -->
  <input
    type="text"
    inputmode="search"
    class="input-sm"
    name="q"
    placeholder="Search"
    autocorrect="off"
    bind:value={searchFormContext.value}
    required
    aria-label="Search"
  />

  <button type="submit" class="submit-button button-reset">
    <span class="sr-only">Search</span>
    <Icon class="i-mdi:magnify" />
  </button>
</form>

<style>
  .form {
    --border-radius: 5000px;
    --submit-icon-size: 1.5em;
    --submit-button-padding-left: var(--space-3xs);
    --submit-button-padding-right: var(--space-2xs);
    --submit-button-width: calc(
      var(--submit-button-padding-left) + var(--submit-icon-size) + var(--submit-button-padding-right)
    );
    --input-padding: var(--space-3xs);
    --input-padding-left: var(--space-xs);
    --input-padding-right: calc(var(--submit-button-width) + var(--input-padding));

    position: relative;
    font-size: calc(var(--space-s) * 0.9);

    &.size-big {
      --input-padding: 8px;
      --input-padding-left: 16px;
      --submit-button-padding-left: 10px;
      --submit-button-padding-right: 13px;

      font-size: var(--space-s);
    }
  }

  .input-lg,
  .input-sm {
    --search-form-focus-shadow: 0 0 0 var(--space-3xs) var(--yellow500);

    border: none;
    color: light-dark(black, var(--main-color));
    background: light-dark(white, hsl(0, 1%, 19%));
    width: 100%;
    padding: var(--input-padding) var(--input-padding-right) var(--input-padding) var(--input-padding-left);
    border-radius: var(--border-radius);
    box-shadow: 1px 2px 4px 0 light-dark(var(--green900), hsl(111, 10%, 8%));
    transition: box-shadow var(--transition-fast);

    &:focus {
      outline: none;
      box-shadow:
        var(--search-form-focus-shadow),
        1px 2px 3px 4px var(--green900);
    }
  }

  .input-lg {
    @media only screen and (max-width: 820px) {
      display: none;
    }
  }

  .input-sm {
    display: none;

    @media only screen and (max-width: 820px) {
      display: unset;
    }
  }

  .submit-button {
    position: absolute;
    /* see https://github.com/rust-lang/crates.io/issues/8677 🤷 */
    right: -1px;
    top: 0;
    bottom: 0;
    display: inline-grid;
    place-items: center;
    padding-left: var(--submit-button-padding-left);
    padding-right: var(--submit-button-padding-right);
    color: white;
    background-color: var(--yellow500);
    border-top-right-radius: var(--border-radius);
    border-bottom-right-radius: var(--border-radius);
    cursor: pointer;

    --icon-size: var(--submit-icon-size);

    &:hover {
      background-color: var(--yellow700);
    }
  }
</style>
