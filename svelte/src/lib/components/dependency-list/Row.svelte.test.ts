import type { components } from '@crates-io/api-client';

import { describe, expect, it } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import Row from './Row.svelte';

type Dependency = components['schemas']['Dependency'];

const HINT = 'This dependency might not be needed anymore.';

const LAZY_STATIC_REPLACEMENT = {
  description: 'Use `std::sync::LazyLock` instead.',
  url: 'https://doc.rust-lang.org/std/sync/struct.LazyLock.html',
};

function createDependency(overrides: Partial<Dependency> = {}): Dependency {
  return {
    id: 1,
    crate_id: 'serde',
    version_id: 42,
    req: '^1.0',
    kind: 'normal',
    optional: false,
    default_features: true,
    features: [],
    downloads: 100_000,
    ...overrides,
  };
}

describe('dependency-list/Row', () => {
  it('shows the native-replacement marker for a superseded dependency', async () => {
    let dependency = createDependency({ crate_id: 'lazy_static' });

    render(Row, {
      dependency,
      descriptionPromise: Promise.resolve(null),
      nativeReplacement: LAZY_STATIC_REPLACEMENT,
    });

    let link = page.getByRole('link', { name: HINT });
    await expect.element(link).toHaveAttribute('href', 'https://doc.rust-lang.org/std/sync/struct.LazyLock.html');
    await expect.element(link).toHaveAttribute('target', '_blank');
    await expect.element(link).toHaveAttribute('rel', 'noopener noreferrer');

    expect(page.getByCSS('[data-test-native-replacement="lazy_static"]').elements()).toHaveLength(1);
  });

  it('shows no marker for a dependency without a replacement', async () => {
    let dependency = createDependency({ crate_id: 'serde' });

    render(Row, { dependency, descriptionPromise: Promise.resolve(null) });

    await expect.element(page.getByCSS('[data-test-crate-name]')).toHaveTextContent('serde');
    expect(page.getByCSS('[data-test-native-replacement]').elements()).toHaveLength(0);
  });
});
