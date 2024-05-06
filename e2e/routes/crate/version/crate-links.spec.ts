import { test, expect } from '@/e2e/helper';

test.describe('Route | crate.version | crate links', { tag: '@routes' }, () => {
  test('shows all external crate links', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', {
        name: 'foo',
        homepage: 'https://crates.io/',
        documentation: 'https://doc.rust-lang.org/cargo/getting-started/',
        repository: 'https://github.com/rust-lang/crates.io.git',
      });
      server.create('version', { crate, num: '1.0.0' });
    });

    await page.goto('/crates/foo');

    const homepageLink = page.locator('[data-test-homepage-link] a');
    const docsLink = page.locator('[data-test-docs-link] a');
    const repositoryLink = page.locator('[data-test-repository-link] a');

    await expect(homepageLink).toHaveText('crates.io');
    await expect(homepageLink).toHaveAttribute('href', 'https://crates.io/');

    await expect(docsLink).toHaveText('doc.rust-lang.org/cargo/getting-started');
    await expect(docsLink).toHaveAttribute('href', 'https://doc.rust-lang.org/cargo/getting-started/');

    await expect(repositoryLink).toHaveText('github.com/rust-lang/crates.io');
    await expect(repositoryLink).toHaveAttribute('href', 'https://github.com/rust-lang/crates.io.git');
  });

  test('shows no external crate links if none are set', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'foo' });
      server.create('version', { crate, num: '1.0.0' });
    });

    await page.goto('/crates/foo');

    await expect(page.locator('[data-test-homepage-link]')).toHaveCount(0);
    await expect(page.locator('[data-test-docs-link]')).toHaveCount(0);
    await expect(page.locator('[data-test-repository-link]')).toHaveCount(0);
  });

  test('hide the homepage link if it is the same as the repository', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', {
        name: 'foo',
        homepage: 'https://github.com/rust-lang/crates.io',
        repository: 'https://github.com/rust-lang/crates.io',
      });
      server.create('version', { crate, num: '1.0.0' });
    });

    await page.goto('/crates/foo');

    await expect(page.locator('[data-test-homepage-link]')).toHaveCount(0);
    await expect(page.locator('[data-test-docs-link]')).toHaveCount(0);

    const repositoryLink = page.locator('[data-test-repository-link] a');
    await expect(repositoryLink).toHaveText('github.com/rust-lang/crates.io');
    await expect(repositoryLink).toHaveAttribute('href', 'https://github.com/rust-lang/crates.io');
  });

  test('hide the homepage link if it is the same as the repository plus `.git`', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', {
        name: 'foo',
        homepage: 'https://github.com/rust-lang/crates.io/',
        repository: 'https://github.com/rust-lang/crates.io.git',
      });
      server.create('version', { crate, num: '1.0.0' });
    });

    await page.goto('/crates/foo');

    await expect(page.locator('[data-test-homepage-link]')).toHaveCount(0);
    await expect(page.locator('[data-test-docs-link]')).toHaveCount(0);

    const repositoryLink = page.locator('[data-test-repository-link] a');
    await expect(repositoryLink).toHaveText('github.com/rust-lang/crates.io');
    await expect(repositoryLink).toHaveAttribute('href', 'https://github.com/rust-lang/crates.io.git');
  });
});
