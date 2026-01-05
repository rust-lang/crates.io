import type { components } from '@crates-io/api-client';

import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import OwnersList from './OwnersList.svelte';

type Owner = components['schemas']['Owner'];

function createUser(id: number, options: { name?: string | null; login?: string } = {}): Owner {
  let login = options.login ?? `user-${id}`;
  let name = options.name === undefined ? `User ${id}` : options.name;
  return {
    id,
    kind: 'user',
    login,
    name,
    avatar: `https://avatars.githubusercontent.com/u/${id}?v=4`,
    url: `https://github.com/${login}`,
  };
}

function createTeam(id: number, org: string, name: string): Owner {
  return {
    id,
    kind: 'team',
    login: `github:${org}:${name}`,
    name,
    avatar: `https://avatars.githubusercontent.com/u/${id}?v=4`,
    url: `https://github.com/${org}`,
  };
}

describe('OwnersList', () => {
  it('single user', async () => {
    let owners: Owner[] = [createUser(1)];

    render(OwnersList, { owners });

    await expect.element(page.getByCSS('[data-test-owners="detailed"]')).toBeVisible();
    await expect.element(page.getByCSS('ul > li')).toBeVisible();
    expect(page.getByCSS('ul > li').elements()).toHaveLength(1);

    let ownerLinks = page.getByCSS('[data-test-owner-link]');
    expect(ownerLinks.elements()).toHaveLength(1);

    let logins = ownerLinks.elements().map(el => el.getAttribute('data-test-owner-link'));
    expect(logins).toEqual(['user-1']);

    let link = page.getByCSS('[data-test-owner-link="user-1"]');
    await expect.element(link).toHaveTextContent('User 1');
    await expect.element(link).toHaveAttribute('href', '/users/user-1');
  });

  it('user without `name`', async () => {
    let owners: Owner[] = [createUser(1, { name: null, login: 'anonymous' })];

    render(OwnersList, { owners });

    await expect.element(page.getByCSS('[data-test-owners="detailed"]')).toBeVisible();
    await expect.element(page.getByCSS('ul > li')).toBeVisible();
    expect(page.getByCSS('ul > li').elements()).toHaveLength(1);

    let ownerLinks = page.getByCSS('[data-test-owner-link]');
    expect(ownerLinks.elements()).toHaveLength(1);

    let logins = ownerLinks.elements().map(el => el.getAttribute('data-test-owner-link'));
    expect(logins).toEqual(['anonymous']);

    let link = page.getByCSS('[data-test-owner-link="anonymous"]');
    await expect.element(link).toHaveTextContent('anonymous');
    await expect.element(link).toHaveAttribute('href', '/users/anonymous');
  });

  it('five users', async () => {
    let owners: Owner[] = Array.from({ length: 5 }, (_, i) => createUser(i + 1));

    render(OwnersList, { owners });

    await expect.element(page.getByCSS('[data-test-owners="detailed"]')).toBeVisible();
    expect(page.getByCSS('ul > li').elements()).toHaveLength(5);
    expect(page.getByCSS('[data-test-owner-link]').elements()).toHaveLength(5);

    let logins = page
      .getByCSS('[data-test-owner-link]')
      .elements()
      .map(el => el.getAttribute('data-test-owner-link'));
    expect(logins).toEqual(['user-1', 'user-2', 'user-3', 'user-4', 'user-5']);
  });

  it('six users', async () => {
    let owners: Owner[] = Array.from({ length: 6 }, (_, i) => createUser(i + 1));

    render(OwnersList, { owners });

    await expect.element(page.getByCSS('[data-test-owners="basic"]')).toBeVisible();
    expect(page.getByCSS('ul > li').elements()).toHaveLength(6);
    expect(page.getByCSS('[data-test-owner-link]').elements()).toHaveLength(6);

    let logins = page
      .getByCSS('[data-test-owner-link]')
      .elements()
      .map(el => el.getAttribute('data-test-owner-link'));
    expect(logins).toEqual(['user-1', 'user-2', 'user-3', 'user-4', 'user-5', 'user-6']);
  });

  it('teams mixed with users', async () => {
    let teams: Owner[] = [createTeam(101, 'crates-io', 'team-1'), createTeam(102, 'crates-io', 'team-2')];
    let users: Owner[] = [createUser(1), createUser(2), createUser(3)];
    let owners: Owner[] = [...teams, ...users];

    render(OwnersList, { owners });

    await expect.element(page.getByCSS('[data-test-owners="detailed"]')).toBeVisible();
    expect(page.getByCSS('ul > li').elements()).toHaveLength(5);
    expect(page.getByCSS('[data-test-owner-link]').elements()).toHaveLength(5);

    let logins = page
      .getByCSS('[data-test-owner-link]')
      .elements()
      .map(el => el.getAttribute('data-test-owner-link'));
    expect(logins).toEqual(['github:crates-io:team-1', 'github:crates-io:team-2', 'user-1', 'user-2', 'user-3']);

    let teamLink = page.getByCSS('[data-test-owner-link="github:crates-io:team-1"]');
    await expect.element(teamLink).toHaveTextContent('crates-io/team-1');
    await expect.element(teamLink).toHaveAttribute('href', '/teams/github:crates-io:team-1');
  });
});
