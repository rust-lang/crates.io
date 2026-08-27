import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import UserPageHeader from './UserPageHeader.svelte';

const USER = {
  avatar: null,
  github_username_matches: true,
  login: 'crates-user',
  name: 'Crates User',
};

const LINKED_ACCOUNTS = [
  {
    account_id: '1',
    avatar: null,
    login: 'github-user',
    provider: 'github' as const,
  },
  {
    account_id: '2',
    avatar: null,
    login: 'crates-user',
    provider: 'github' as const,
  },
];

describe('UserPageHeader', () => {
  it('renders every linked account', async () => {
    await render(UserPageHeader, { user: USER, linkedAccounts: LINKED_ACCOUNTS });

    let chips = page.getByCSS('[data-test-account-chip]');
    expect(chips.elements()).toHaveLength(2);
    expect(chips.elements().map(chip => chip.getAttribute('href'))).toEqual([
      'https://github.com/github-user',
      'https://github.com/crates-user',
    ]);
  });

  it('does not mark GitHub accounts when one matches', async () => {
    await render(UserPageHeader, { user: USER, linkedAccounts: LINKED_ACCOUNTS });

    expect(page.getByCSS('[data-test-mismatch-marker]').elements()).toHaveLength(0);
  });

  it('marks every GitHub account when none match', async () => {
    await render(UserPageHeader, {
      user: { ...USER, github_username_matches: false },
      linkedAccounts: LINKED_ACCOUNTS,
    });

    expect(page.getByCSS('[data-test-mismatch-marker]').elements()).toHaveLength(2);
  });

  it('omits the account row when there are no linked accounts', async () => {
    await render(UserPageHeader, { user: USER, linkedAccounts: [] });

    expect(page.getByCSS('.accounts').elements()).toHaveLength(0);
  });
});
