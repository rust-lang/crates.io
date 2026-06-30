import { afterEach, describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import SearchFormTestWrapper from './SearchFormTestWrapper.svelte';

function pressKey(target: EventTarget, key: string) {
  target.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, composed: true }));
}

describe('SearchForm', () => {
  let cleanup: (() => void) | undefined;

  afterEach(() => {
    cleanup?.();
    cleanup = undefined;
  });

  it('focuses the search input when pressing "S" outside any input', async () => {
    // The large input carrying `data-test-search-input` is hidden below 820px,
    // so widen the viewport to make it focusable.
    await page.viewport(1280, 800);
    render(SearchFormTestWrapper);
    let input = page.getByCSS('[data-test-search-input]').element() as HTMLInputElement;
    expect(document.activeElement).not.toBe(input);

    pressKey(document.body, 'S');

    expect(document.activeElement).toBe(input);
  });

  it('does not steal focus while typing in an input inside a shadow root', () => {
    render(SearchFormTestWrapper);
    let searchInput = page.getByCSS('[data-test-search-input]').element() as HTMLInputElement;

    // An input inside a shadow root: a keydown from it is retargeted to the
    // shadow host by the time it reaches the window listener.
    let host = document.createElement('div');
    document.body.append(host);
    let shadowInput = document.createElement('input');
    host.attachShadow({ mode: 'open' }).append(shadowInput);
    shadowInput.focus();
    cleanup = () => host.remove();

    pressKey(shadowInput, 'S');

    expect(document.activeElement).toBe(host);
    expect(document.activeElement).not.toBe(searchInput);
  });
});
