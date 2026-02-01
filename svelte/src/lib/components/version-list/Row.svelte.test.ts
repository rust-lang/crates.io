import type { components } from '@crates-io/api-client';

import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import Row from './Row.svelte';

type Version = components['schemas']['Version'];

// TODO this is supposed to just be `Partial<Version>` but TS is currently unhappy
// with the type signature of some of the fields derived from the OpenAPI spec.
interface VersionOverrides extends Partial<Omit<Version, 'features'>> {
  features?: Record<string, string[]>;
}

function createVersion(overrides: VersionOverrides = {}): Version {
  return {
    id: 1,
    crate: 'foo',
    num: '1.0.0',
    created_at: new Date().toISOString(),
    downloads: 1000,
    crate_size: 10000,
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
  it('handle non-standard semver strings', async () => {
    let firstVersion = createVersion({ num: '0.4.0-alpha.01' });
    let secondVersion = createVersion({ num: '0.3.0-alpha.01' });

    let { unmount } = render(Row, { version: firstVersion, crateName: 'foo' });
    await expect.element(page.getByCSS('[data-test-release-track]')).toHaveTextContent('0.4');
    await expect.element(page.getByCSS('[data-test-release-track-link]')).toHaveTextContent('0.4.0-alpha.01');
    unmount();

    render(Row, { version: secondVersion, crateName: 'foo' });
    await expect.element(page.getByCSS('[data-test-release-track]')).toHaveTextContent('0.3');
    await expect.element(page.getByCSS('[data-test-release-track-link]')).toHaveTextContent('0.3.0-alpha.01');
  });

  it('handle node-semver parsing errors', async () => {
    let num = '18446744073709551615.18446744073709551615.18446744073709551615';
    let version = createVersion({ num });

    render(Row, { version, crateName: 'foo' });
    await expect.element(page.getByCSS('[data-test-release-track]')).toHaveTextContent('?');
    await expect.element(page.getByCSS('[data-test-release-track-link]')).toHaveTextContent(num);
  });

  it('pluralize "feature" only when appropriate', async () => {
    let firstVersion = createVersion({ num: '0.1.0', features: {} });
    let secondVersion = createVersion({ num: '0.2.0', features: { one: [] } });
    let thirdVersion = createVersion({ num: '0.3.0', features: { one: [], two: [] } });

    let { unmount: unmount1 } = render(Row, { version: firstVersion, crateName: 'foo' });
    expect(page.getByCSS('[data-test-feature-list]').query()).toBeNull();
    unmount1();

    let { unmount: unmount2 } = render(Row, { version: secondVersion, crateName: 'foo' });
    await expect.element(page.getByCSS('[data-test-feature-list]')).toHaveTextContent('1 Feature');
    unmount2();

    render(Row, { version: thirdVersion, crateName: 'foo' });
    await expect.element(page.getByCSS('[data-test-feature-list]')).toHaveTextContent('2 Features');
  });
});
