import type { components } from '@crates-io/api-client';

import { http, HttpResponse } from 'msw';
import { describe, expect, vi } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page } from 'vitest/browser';

import { defer } from '$lib/utils/deferred';
import { test } from '../../test/msw';
import YankButtonTestWrapper from './YankButtonTestWrapper.svelte';

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

describe('YankButton', () => {
  describe('yanking', () => {
    test('yanks a version successfully', async ({ worker }) => {
      let version = createVersion({ yanked: false });
      let onChanged = vi.fn();
      let deferred = defer<Response>();

      worker.use(http.delete('/api/v1/crates/:name/:version/yank', () => deferred.promise));

      render(YankButtonTestWrapper, { crateName: 'foo', version, onChanged });

      await expect.element(page.getByCSS('[data-test-version-yank-button="1.0.0"]')).toHaveTextContent('Yank');
      await expect.element(page.getByCSS('[data-test-version-unyank-button]')).not.toBeInTheDocument();

      await page.getByCSS('[data-test-version-yank-button="1.0.0"]').click();
      await expect.element(page.getByCSS('[data-test-version-yank-button="1.0.0"]')).toHaveTextContent('Yanking...');
      await expect.element(page.getByCSS('[data-test-version-yank-button="1.0.0"]')).toBeDisabled();
      expect(onChanged).not.toHaveBeenCalled();

      deferred.resolve(HttpResponse.json({ ok: true }));
      await expect.element(page.getByCSS('[data-test-version-unyank-button="1.0.0"]')).toHaveTextContent('Unyank');
      await expect.element(page.getByCSS('[data-test-version-yank-button]')).not.toBeInTheDocument();
      expect(onChanged).toHaveBeenCalledOnce();
    });

    test('keeps version unyanked on API error', async ({ worker }) => {
      let version = createVersion({ yanked: false });
      let onChanged = vi.fn();

      worker.use(http.delete('/api/v1/crates/:name/:version/yank', () => HttpResponse.json({}, { status: 500 })));

      render(YankButtonTestWrapper, { crateName: 'foo', version, onChanged });

      await page.getByCSS('[data-test-version-yank-button="1.0.0"]').click();

      expect(onChanged).not.toHaveBeenCalled();
      await expect.element(page.getByCSS('[data-test-version-yank-button="1.0.0"]')).toHaveTextContent('Yank');
      await expect.element(page.getByCSS('[data-test-version-yank-button="1.0.0"]')).toBeEnabled();
    });
  });

  describe('unyanking', () => {
    test('unyanks a version successfully', async ({ worker }) => {
      let version = createVersion({ yanked: true });
      let onChanged = vi.fn();
      let deferred = defer<Response>();

      worker.use(http.put('/api/v1/crates/:name/:version/unyank', () => deferred.promise));

      render(YankButtonTestWrapper, { crateName: 'foo', version, onChanged });

      await expect.element(page.getByCSS('[data-test-version-unyank-button="1.0.0"]')).toHaveTextContent('Unyank');
      await expect.element(page.getByCSS('[data-test-version-yank-button]')).not.toBeInTheDocument();

      await page.getByCSS('[data-test-version-unyank-button="1.0.0"]').click();
      await expect
        .element(page.getByCSS('[data-test-version-unyank-button="1.0.0"]'))
        .toHaveTextContent('Unyanking...');
      await expect.element(page.getByCSS('[data-test-version-unyank-button="1.0.0"]')).toBeDisabled();
      expect(onChanged).not.toHaveBeenCalled();

      deferred.resolve(HttpResponse.json({ ok: true }));
      await expect.element(page.getByCSS('[data-test-version-yank-button="1.0.0"]')).toHaveTextContent('Yank');
      await expect.element(page.getByCSS('[data-test-version-unyank-button]')).not.toBeInTheDocument();
      expect(onChanged).toHaveBeenCalledOnce();
    });

    test('keeps version yanked on API error', async ({ worker }) => {
      let version = createVersion({ yanked: true });
      let onChanged = vi.fn();

      worker.use(http.put('/api/v1/crates/:name/:version/unyank', () => HttpResponse.json({}, { status: 500 })));

      render(YankButtonTestWrapper, { crateName: 'foo', version, onChanged });

      await page.getByCSS('[data-test-version-unyank-button="1.0.0"]').click();

      expect(onChanged).not.toHaveBeenCalled();
      await expect.element(page.getByCSS('[data-test-version-unyank-button="1.0.0"]')).toHaveTextContent('Unyank');
      await expect.element(page.getByCSS('[data-test-version-unyank-button="1.0.0"]')).toBeEnabled();
    });
  });
});
