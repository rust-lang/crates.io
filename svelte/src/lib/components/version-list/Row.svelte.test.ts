import type { components } from '@crates-io/api-client';

import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import Row from './RowTestWrapper.svelte';

type Version = components['schemas']['Version'];

function createVersion(overrides: Partial<Version> = {}): Version {
  return {
    id: 1,
    crate: 'foo',
    num: '1.0.0',
    created_at: new Date().toISOString(),
    downloads: 1000,
    crate_size: 10_000,
    license: 'MIT',
    rust_version: null,
    edition: null,
    yanked: false,
    features: {},
    checksum: 'abc123',
    dl_path: '/api/v1/crates/foo/1.0.0/download',
    readme_path: '/api/v1/crates/foo/1.0.0/readme',
    updated_at: new Date().toISOString(),
    audit_actions: [],
    links: {
      authors: '/api/v1/crates/foo/1.0.0/authors',
      dependencies: '/api/v1/crates/foo/1.0.0/dependencies',
      version_downloads: '/api/v1/crates/foo/1.0.0/downloads',
    },
    linecounts: {},
    published_by: null,
    ...overrides,
  } as Version;
}

describe('version-list/Row', () => {
  let track = page.getByCSS('[data-test-release-track]');
  let trackLabel = page.getByCSS('[data-test-release-track] [aria-hidden="true"]');
  let trackSrLabel = page.getByCSS('[data-test-release-track] .sr-only');
  let versionLink = page.getByCSS('[data-test-release-track-link]');
  let tooltip = page.getByCSS('.tooltip');

  it('handle non-standard semver strings', async () => {
    let firstVersion = createVersion({ num: '0.4.0-alpha.01' });
    let secondVersion = createVersion({ num: '0.3.0-alpha.01' });

    let { unmount } = await render(Row, { version: firstVersion, crateName: 'foo' });
    await expect.element(trackLabel).toHaveTextContent('0.4');
    await expect.element(trackSrLabel).toHaveTextContent('Release track 0.4 (prerelease)');
    await expect.element(versionLink).toHaveTextContent('Version 0.4.0-alpha.01');

    await userEvent.hover(track);
    await expect.element(tooltip).toHaveTextContent('Release Track: 0.4 (prerelease)');
    await unmount();

    await render(Row, { version: secondVersion, crateName: 'foo' });
    await expect.element(trackLabel).toHaveTextContent('0.3');
    await expect.element(trackSrLabel).toHaveTextContent('Release track 0.3 (prerelease)');
    await expect.element(versionLink).toHaveTextContent('Version 0.3.0-alpha.01');

    await userEvent.hover(track);
    await expect.element(tooltip).toHaveTextContent('Release Track: 0.3 (prerelease)');
  });

  it('handle node-semver parsing errors', async () => {
    let num = '18446744073709551615.18446744073709551615.18446744073709551615';
    let version = createVersion({ num });

    await render(Row, { version, crateName: 'foo' });
    await expect.element(trackLabel).toHaveTextContent('?');
    await expect.element(trackSrLabel).toHaveTextContent('Unable to parse version');
    await expect.element(versionLink).toHaveTextContent(`Version ${num}`);

    await userEvent.hover(track);
    await expect.element(tooltip).toHaveTextContent(`Failed to parse version ${num}`);
  });

  it('labels a pre-release that is the highest of its release track', async () => {
    let version = createVersion({ num: '2.0.0-beta.1' });

    await render(Row, { version, crateName: 'foo', isHighestOfReleaseTrack: true });
    await expect.element(trackSrLabel).toHaveTextContent('Release track 2.x (prerelease, latest)');

    await userEvent.hover(track);
    await expect.element(tooltip).toHaveTextContent('Release Track: 2.x (prerelease, latest)');
  });

  it('pluralize "feature" only when appropriate', async () => {
    let firstVersion = createVersion({ num: '0.1.0', features: {} });
    let secondVersion = createVersion({ num: '0.2.0', features: { one: [] } });
    let thirdVersion = createVersion({ num: '0.3.0', features: { one: [], two: [] } });

    let { unmount: unmount1 } = await render(Row, { version: firstVersion, crateName: 'foo' });
    expect(page.getByCSS('[data-test-feature-list]').query()).toBeNull();
    await unmount1();

    let { unmount: unmount2 } = await render(Row, { version: secondVersion, crateName: 'foo' });
    await expect.element(page.getByCSS('[data-test-feature-list]')).toHaveTextContent('1 Feature');
    await unmount2();

    await render(Row, { version: thirdVersion, crateName: 'foo' });
    await expect.element(page.getByCSS('[data-test-feature-list]')).toHaveTextContent('2 Features');
  });
});
