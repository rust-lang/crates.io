<script lang="ts">
  import type { ColorScheme } from '$lib/color-scheme.svelte';
  import type { HTMLAttributes } from 'svelte/elements';

  import { getColorScheme } from '$lib/color-scheme.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import * as Dropdown from './dropdown';

  type Props = HTMLAttributes<HTMLDivElement>;

  let { class: className, ...restProps }: Props = $props();

  interface SchemeOption {
    mode: ColorScheme;
    iconClass: string;
  }

  const COLOR_SCHEMES: SchemeOption[] = [
    { mode: 'light', iconClass: 'i-heroicons:sun' },
    { mode: 'dark', iconClass: 'i-heroicons:moon' },
    { mode: 'system', iconClass: 'i-mdi:circle-half-full' },
  ];

  let colorScheme = getColorScheme();

  let currentIconClass = $derived(
    COLOR_SCHEMES.find(({ mode }) => mode === colorScheme.scheme)?.iconClass ?? 'i-heroicons:sun',
  );
</script>

<div class={['color-scheme-menu', className]} {...restProps}>
  <Dropdown.Root>
    <Dropdown.Trigger hideArrow class="trigger">
      <Icon class={currentIconClass} />
      <span class="sr-only">Change color scheme. Current: {colorScheme.scheme}</span>
    </Dropdown.Trigger>

    <Dropdown.Menu class="menu">
      {#each COLOR_SCHEMES as { mode, iconClass } (mode)}
        <Dropdown.Item>
          <button
            class="menu-button button-reset"
            class:selected={mode === colorScheme.scheme}
            type="button"
            onclick={() => colorScheme.setScheme(mode)}
          >
            <Icon class={iconClass} />
            {mode}
          </button>
        </Dropdown.Item>
      {/each}
    </Dropdown.Menu>
  </Dropdown.Root>
</div>

<style>
  .color-scheme-menu {
    display: flex;
    align-items: center;

    & :global(.icon) {
      width: 1.4em;
      height: 1.4em;
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
