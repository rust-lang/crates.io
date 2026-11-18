import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import UsernameMismatchNoteTestWrapper from './UsernameMismatchNoteTestWrapper.svelte';

const TOOLTIP_TEXT = 'bobsmith on crates.io is not linked to bobsmith on GitHub.';

describe('UsernameMismatchNote', () => {
  it('adds the mismatch to the accessible name of the surrounding link', async () => {
    await render(UsernameMismatchNoteTestWrapper, { name: 'Bob Smith', login: 'bobsmith' });

    await expect
      .element(page.getByRole('link', { name: 'Bob Smith (not linked to GitHub user bobsmith)', exact: true }))
      .toBeVisible();
  });

  it('explains the mismatch in a tooltip on hover', async () => {
    await render(UsernameMismatchNoteTestWrapper, { name: 'Bob Smith', login: 'bobsmith' });

    await userEvent.hover(page.getByRole('link'));

    await expect.element(page.getByCSS('.tooltip')).toHaveTextContent(TOOLTIP_TEXT);
  });

  it('explains the mismatch in a tooltip on focus', async () => {
    await render(UsernameMismatchNoteTestWrapper, { name: 'Bob Smith', login: 'bobsmith' });

    page.getByRole('link').element().focus();

    await expect.element(page.getByCSS('.tooltip')).toHaveTextContent(TOOLTIP_TEXT);
  });
});
