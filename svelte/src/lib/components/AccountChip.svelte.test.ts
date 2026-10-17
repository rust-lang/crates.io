import { tick } from 'svelte';
import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import AccountChipTestWrapper from './AccountChipTestWrapper.svelte';

describe('AccountChip', () => {
  it('renders a GitHub account link', async () => {
    await render(AccountChipTestWrapper, {
      handle: 'tbieniek',
      href: 'https://github.com/tbieniek',
    });

    let chip = page.getByCSS('[data-test-account-chip]');
    await expect.element(chip).toHaveAttribute('href', 'https://github.com/tbieniek');
    await expect.element(page.getByCSS('[data-test-provider-icon]')).toHaveAttribute('aria-label', 'GitHub');
    await expect.element(page.getByCSS('[data-test-handle]')).toHaveTextContent('tbieniek');
    await expect.element(chip).toHaveAccessibleName('GitHub tbieniek');
    await expect.element(chip).not.toHaveAttribute('aria-describedby');
  });

  it('does not show a tooltip when the matching handle fits', async () => {
    await render(AccountChipTestWrapper, {
      handle: 'tbieniek',
      href: 'https://github.com/tbieniek',
      width: '500px',
    });

    await userEvent.hover(page.getByCSS('[data-test-account-chip]'));
    await tick();

    expect(document.querySelector('.tooltip')).toBeNull();
  });

  it('shows the complete matching handle when it is truncated', async () => {
    let handle = 'averylonggithubhandlethatdoesnotfit';
    await render(AccountChipTestWrapper, {
      handle,
      href: `https://github.com/${handle}`,
      width: '150px',
    });

    await userEvent.hover(page.getByCSS('[data-test-account-chip]'));

    await expect.element(page.getByCSS('.tooltip')).toHaveTextContent(handle);
  });

  it('marks a mismatch and explains it in the tooltip', async () => {
    let description = 'This crates.io account may differ from the GitHub account named "tbieniek".';
    await render(AccountChipTestWrapper, {
      handle: 'Turbo87',
      href: 'https://github.com/Turbo87',
      mismatchedUsername: 'tbieniek',
    });

    let marker = page.getByCSS('[data-test-mismatch-marker]');
    await expect.element(marker).toHaveTextContent('≠');
    await expect.element(marker).toHaveAttribute('aria-hidden', 'true');
    await expect.element(page.getByCSS('[data-test-mismatch-description]')).toHaveTextContent(description);
    let chip = page.getByCSS('[data-test-account-chip]');
    await expect.element(chip).toHaveAccessibleName('GitHub Turbo87');
    await expect.element(chip).toHaveAccessibleDescription(description);

    await userEvent.hover(page.getByCSS('[data-test-account-chip]'));

    await expect.element(page.getByCSS('.tooltip')).toHaveTextContent(`Turbo87 ${description}`);
    await expect.element(chip).toHaveAccessibleName('GitHub Turbo87');
  });

  it('shows the complete mismatched handle and warning on focus', async () => {
    let handle = 'averylonggithubhandlethatdoesnotfit';
    await render(AccountChipTestWrapper, {
      handle,
      href: `https://github.com/${handle}`,
      mismatchedUsername: 'tbieniek',
      width: '150px',
    });

    let handleElement = page.getByCSS('[data-test-handle]').element();
    expect(handleElement.scrollWidth).toBeGreaterThan(handleElement.clientWidth);
    let chip = page.getByCSS('[data-test-account-chip]');
    (chip.element() as HTMLAnchorElement).focus();
    await expect.element(chip).toHaveFocus();
    await expect.element(chip).toHaveAccessibleName(`GitHub ${handle}`);
    await expect
      .element(page.getByCSS('.tooltip'))
      .toHaveTextContent(`${handle} This crates.io account may differ from the GitHub account named "tbieniek".`);
  });
});
