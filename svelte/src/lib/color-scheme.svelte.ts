import { createContext } from 'svelte';
import { MediaQuery } from 'svelte/reactivity';

import * as storage from '$lib/utils/local-storage';

export type ColorScheme = 'light' | 'dark' | 'system';
export type ResolvedScheme = 'light' | 'dark';

const STORAGE_KEY = 'color-scheme';
const VALID_SCHEMES = new Set<ColorScheme>(['light', 'dark', 'system']);

export class ColorSchemeState {
  scheme = $state<ColorScheme>('system');

  #mediaQuery = new MediaQuery('prefers-color-scheme: dark', false);

  readonly resolvedScheme: ResolvedScheme = $derived(
    this.scheme === 'system' ? (this.#mediaQuery.current ? 'dark' : 'light') : this.scheme,
  );

  readonly isDark: boolean = $derived(this.resolvedScheme === 'dark');

  constructor() {
    let stored = storage.getItem(STORAGE_KEY);
    if (stored && VALID_SCHEMES.has(stored as ColorScheme)) {
      this.scheme = stored as ColorScheme;
    }
  }

  setScheme(newScheme: ColorScheme): void {
    if (!VALID_SCHEMES.has(newScheme)) return;

    this.scheme = newScheme;
    storage.setItem(STORAGE_KEY, newScheme);
  }
}

export interface ColorSchemeContext {
  readonly scheme: ColorScheme;
  readonly resolvedScheme: ResolvedScheme;
  readonly isDark: boolean;
  setScheme: (scheme: ColorScheme) => void;
}

export const [getColorScheme, setColorScheme] = createContext<ColorSchemeContext>();
