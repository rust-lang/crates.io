import type { components } from '@crates-io/api-client';
import type { PlaygroundCrate } from '$lib/utils/playground';

import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import { defer } from '$lib/utils/deferred';
import CrateSidebarTestWrapper from './CrateSidebarTestWrapper.svelte';

type Crate = components['schemas']['Crate'];
type Version = components['schemas']['Version'];
type Owner = components['schemas']['Owner'];

function createCrate(name: string): Crate {
  return {
    id: name,
    name,
    created_at: '2020-01-01T00:00:00Z',
    updated_at: '2020-01-01T00:00:00Z',
    downloads: 1000,
    default_version: '1.0.0',
    num_versions: 1,
    max_version: '1.0.0',
    max_stable_version: '1.0.0',
    newest_version: '1.0.0',
    description: `A test crate called ${name}`,
    homepage: null,
    documentation: null,
    repository: null,
    yanked: false,
    badges: [],
    exact_match: false,
    trustpub_only: false,
    links: {
      owner_team: `/api/v1/crates/${name}/owner_team`,
      owner_user: `/api/v1/crates/${name}/owner_user`,
      reverse_dependencies: `/api/v1/crates/${name}/reverse_dependencies`,
      version_downloads: `/api/v1/crates/${name}/downloads`,
      versions: `/api/v1/crates/${name}/versions`,
    },
  };
}

function createVersion(num: string): Version {
  return {
    id: 1,
    crate: 'test-crate',
    num,
    created_at: '2020-01-01T00:00:00Z',
    updated_at: '2020-01-01T00:00:00Z',
    downloads: 500,
    yanked: false,
    license: 'MIT',
    crate_size: 10000,
    published_by: null,
    rust_version: null,
    edition: '2021',
    has_lib: true,
    bin_names: [],
    linecounts: {},
    checksum: 'abc123',
    readme_path: `/api/v1/crates/test-crate/${num}/readme`,
    dl_path: `/api/v1/crates/test-crate/${num}/download`,
    features: {},
    links: {
      authors: `/api/v1/crates/test-crate/${num}/authors`,
      dependencies: `/api/v1/crates/test-crate/${num}/dependencies`,
      version_downloads: `/api/v1/crates/test-crate/${num}/downloads`,
    },
    audit_actions: [],
  };
}

function createOwner(id: number): Owner {
  return {
    id,
    kind: 'user',
    login: `user-${id}`,
    name: `User ${id}`,
    avatar: `https://avatars.githubusercontent.com/u/${id}?v=4`,
    url: `https://github.com/user-${id}`,
  };
}

const PLAYGROUND_CRATES: PlaygroundCrate[] = [
  { name: 'addr2line', version: '0.14.1', id: 'addr2line' },
  { name: 'adler', version: '0.2.3', id: 'adler' },
  { name: 'adler32', version: '1.2.0', id: 'adler32' },
  { name: 'ahash', version: '0.4.7', id: 'ahash' },
  { name: 'aho-corasick', version: '0.7.15', id: 'aho_corasick' },
  { name: 'ansi_term', version: '0.12.1', id: 'ansi_term' },
];

describe('CrateSidebar Playground Button', () => {
  it('button is hidden for unavailable crates', async () => {
    let crate = createCrate('foo');
    let version = createVersion('1.0.0');
    let owners = [createOwner(1)];
    let playgroundCratesPromise = Promise.resolve(PLAYGROUND_CRATES);

    render(CrateSidebarTestWrapper, { crate, version, owners, playgroundCratesPromise });

    // Button should not exist for crates not in the playground list
    expect(page.getByCSS('[data-test-playground-button]').query()).toBeNull();
  });

  it('button is visible for available crates', async () => {
    let crate = createCrate('aho-corasick');
    let version = createVersion('1.0.0');
    let owners = [createOwner(1)];
    let playgroundCratesPromise = Promise.resolve(PLAYGROUND_CRATES);

    render(CrateSidebarTestWrapper, { crate, version, owners, playgroundCratesPromise });

    let expectedHref =
      'https://play.rust-lang.org/?edition=2021&code=use%20aho_corasick%3B%0A%0Afn%20main()%20%7B%0A%20%20%20%20%2F%2F%20try%20using%20the%20%60aho_corasick%60%20crate%20here%0A%7D';

    await expect.element(page.getByCSS('[data-test-playground-button]')).toBeVisible();
    await expect.element(page.getByCSS('[data-test-playground-button]')).toHaveAttribute('href', expectedHref);
  });

  it('button is hidden while Playground request is pending', async () => {
    let crate = createCrate('aho-corasick');
    let version = createVersion('1.0.0');
    let owners = [createOwner(1)];
    let deferred = defer<PlaygroundCrate[]>();

    render(CrateSidebarTestWrapper, { crate, version, owners, playgroundCratesPromise: deferred.promise });

    await expect.element(page.getByCSS('[data-test-owners]')).toBeVisible();

    // Button should not exist while the request is pending
    expect(page.getByCSS('[data-test-playground-button]').query()).toBeNull();

    // Resolve the promise to clean up
    deferred.resolve(PLAYGROUND_CRATES);
  });

  it('button is hidden if the Playground request fails', async () => {
    let crate = createCrate('aho-corasick');
    let version = createVersion('1.0.0');
    let owners = [createOwner(1)];
    let playgroundCratesPromise = Promise.reject(new Error('Failed to load'));

    render(CrateSidebarTestWrapper, { crate, version, owners, playgroundCratesPromise });

    // Button should not exist when the request fails
    expect(page.getByCSS('[data-test-playground-button]').query()).toBeNull();
  });
});
