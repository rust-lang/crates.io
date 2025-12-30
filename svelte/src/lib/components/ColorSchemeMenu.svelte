<script lang="ts">
  import type { ColorScheme } from '$lib/color-scheme.svelte';
  import type { Component } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  import ColorModeIcon from '$lib/assets/color-mode.svg?component';
  import MoonIcon from '$lib/assets/moon.svg?component';
  import SunIcon from '$lib/assets/sun.svg?component';
  import { getColorScheme } from '$lib/color-scheme.svelte';
  import * as Dropdown from './dropdown';

  type Props = HTMLAttributes<HTMLDivElement>;

  let { class: className, ...restProps }: Props = $props();

  interface SchemeOption {
    mode: ColorScheme;
    Icon: Component;
  }

  const COLOR_SCHEMES: SchemeOption[] = [
    { mode: 'light', Icon: SunIcon },
    { mode: 'dark', Icon: MoonIcon },
    { mode: 'system', Icon: ColorModeIcon },
  ];

  let colorScheme = getColorScheme();

  let CurrentIcon: Component = $derived(COLOR_SCHEMES.find(({ mode }) => mode === colorScheme.scheme)?.Icon ?? SunIcon);
</script>

<div class={['color-scheme-menu', className]} {...restProps}>
  <Dropdown.Root class="dropdown">
    <Dropdown.Trigger hideArrow class="trigger">
      <CurrentIcon class="icon" />
      <span class="sr-only">Change color scheme</span>
    </Dropdown.Trigger>

    <Dropdown.Menu class="menu">
      {#each COLOR_SCHEMES as { mode, Icon } (mode)}
        <Dropdown.Item>
          <button
            class="menu-button button-reset"
            class:selected={mode === colorScheme.scheme}
            type="button"
            onclick={() => colorScheme.setScheme(mode)}
          >
            <Icon class="icon" />
            {mode}
          </button>
        </Dropdown.Item>
      {/each}
    </Dropdown.Menu>
  </Dropdown.Root>
</div>

<style>
  .color-scheme-menu {
    & :global(.dropdown) {
      line-height: 1rem;
    }

    & :global(.icon) {
      width: 1.4em;
      height: auto;
    }

    & :global(.trigger) {
      background: none;
      border: 0;
      padding: 0;
    }

    & :global(.menu) {
      right: 0;
      min-width: max-content;
    }

    .menu-button {
      align-items: center;
      gap: var(--space-2xs);
      cursor: pointer;
      text-transform: capitalize;
    }

    .selected {
      background: light-dark(#e6e6e6, #404040);
    }
  }
</style>
