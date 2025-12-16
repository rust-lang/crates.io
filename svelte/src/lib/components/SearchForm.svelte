<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';

  import SearchIcon from '$lib/assets/search.svg?component';

  interface Props {
    size?: 'big';
    autofocus?: boolean;
  }

  let { size, autofocus = false }: Props = $props();

  // TODO: move search state into header context instead
  let searchValue = $state('');
  let inputElement: HTMLInputElement | undefined = $state();

  function updateSearchValue(event: Event) {
    searchValue = (event.target as HTMLInputElement).value;
  }

  function search(event: SubmitEvent) {
    event.preventDefault();
    // eslint-disable-next-line svelte/no-navigation-without-resolve -- resolve() doesn't support query params
    goto(`${resolve('/search')}?q=${encodeURIComponent(searchValue)}&page=1`);
  }

  function handleKeydown(event: KeyboardEvent) {
    // Don't trigger if user is typing in an input/textarea or if modifier keys are pressed
    let target = event.target as HTMLElement;
    if (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable) {
      return;
    }
    if (event.ctrlKey || event.altKey || event.metaKey) {
      return;
    }

    if (event.key === 's' || event.key === 'S' || event.key === '/') {
      event.preventDefault();
      inputElement?.focus();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_autofocus -->
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
    id="cargo-desktop-search"
    placeholder="Type 'S' or '/' to search"
    autocorrect="off"
    value={searchValue}
    oninput={updateSearchValue}
    {autofocus}
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
    value={searchValue}
    oninput={updateSearchValue}
    required
    aria-label="Search"
  />

  <button type="submit" class="submit-button button-reset">
    <span class="sr-only">Submit</span>
    <SearchIcon />
  </button>
</form>

<style>
  .form {
    --border-radius: 5000px;
    --submit-icon-size: 1em;
    --submit-button-padding-left: var(--space-2xs);
    --submit-button-padding-right: var(--space-xs);
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
      --submit-button-padding-left: 12px;
      --submit-button-padding-right: 16px;

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
    right: -0.5px;
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

    &:hover {
      background-color: var(--yellow700);
    }
  }

  .submit-button :global(svg) {
    width: var(--submit-icon-size);
    height: var(--submit-icon-size);
  }
</style>
