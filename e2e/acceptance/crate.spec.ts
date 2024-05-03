import { test, expect } from '@/e2e/helper';

test.describe('Acceptance | crate page', { tag: '@acceptance' }, () => {
  test('visiting a crate page from the front page', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'nanomsg', newest_version: '0.6.1' });
      server.create('version', { crate, num: '0.6.1' });
    });

    await page.goto('/');
    await page.click('[data-test-just-updated] [data-test-crate-link="0"]');

    await expect(page).toHaveURL('/crates/nanomsg/0.6.1');
    await expect(page).toHaveTitle('nanomsg - crates.io: Rust Package Registry');

    await expect(page.locator('[data-test-heading] [data-test-crate-name]')).toHaveText('nanomsg');
    await expect(page.locator('[data-test-heading] [data-test-crate-version]')).toHaveText('v0.6.1');
  });

  test('visiting /crates/nanomsg', async ({ page, mirage, ember, percy, a11y }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'nanomsg' });
      server.create('version', { crate, num: '0.6.0' });
      server.create('version', { crate, num: '0.6.1', rust_version: '1.69' });
    });

    await page.goto('/crates/nanomsg');

    await expect(page).toHaveURL('/crates/nanomsg');
    await expect(page).toHaveTitle('nanomsg - crates.io: Rust Package Registry');
    // TODO: Add the following as a method to EmberPage fixture
    const currentRouteName = await ember.evaluate(owner => owner.lookup('router:main').currentRouteName);
    expect(currentRouteName).toBe('crate.index');

    await expect(page.locator('[data-test-heading] [data-test-crate-name]')).toHaveText('nanomsg');
    await expect(page.locator('[data-test-heading] [data-test-crate-version]')).toHaveText('v0.6.1');
    await expect(page.locator('[data-test-crate-stats-label]')).toHaveText('Stats Overview');

    await percy.snapshot();
    await a11y.audit();
  });

  test('visiting /crates/nanomsg/', async ({ page, mirage, ember }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'nanomsg' });
      server.create('version', { crate, num: '0.6.0' });
      server.create('version', { crate, num: '0.6.1' });
    });

    await page.goto('/crates/nanomsg/');

    await expect(page).toHaveURL('/crates/nanomsg/');
    await expect(page).toHaveTitle('nanomsg - crates.io: Rust Package Registry');
    // TODO: Add the following as a method to EmberPage fixture
    const currentRouteName = await ember.evaluate(owner => owner.lookup('router:main').currentRouteName);
    expect(currentRouteName).toBe('crate.index');

    await expect(page.locator('[data-test-heading] [data-test-crate-name]')).toHaveText('nanomsg');
    await expect(page.locator('[data-test-heading] [data-test-crate-version]')).toHaveText('v0.6.1');
    await expect(page.locator('[data-test-crate-stats-label]')).toHaveText('Stats Overview');
  });

  test('visiting /crates/nanomsg/0.6.0', async ({ page, mirage, ember, percy, a11y }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'nanomsg' });
      server.create('version', { crate, num: '0.6.0' });
      server.create('version', { crate, num: '0.6.1' });
    });

    await page.goto('/crates/nanomsg/0.6.0');

    await expect(page).toHaveURL('/crates/nanomsg/0.6.0');
    await expect(page).toHaveTitle('nanomsg - crates.io: Rust Package Registry');
    // TODO: Add the following as a method to EmberPage fixture
    const currentRouteName = await ember.evaluate(owner => owner.lookup('router:main').currentRouteName);
    expect(currentRouteName).toBe('crate.version');

    await expect(page.locator('[data-test-heading] [data-test-crate-name]')).toHaveText('nanomsg');
    await expect(page.locator('[data-test-heading] [data-test-crate-version]')).toHaveText('v0.6.0');
    await expect(page.locator('[data-test-crate-stats-label]')).toHaveText('Stats Overview for 0.6.0 (see all)');

    await percy.snapshot();
    await a11y.audit();
  });

  test('unknown crate shows an error message', async ({ page }) => {
    await page.goto('/crates/nanomsg');
    await expect(page).toHaveURL('/crates/nanomsg');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('nanomsg: Crate not found');
    await expect(page.locator('[data-test-go-back]')).toBeVisible();
    await expect(page.locator('[data-test-try-again]')).toHaveCount(0);
  });

  test('other crate loading error shows an error message', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.get('/api/v1/crates/:crate_name', {}, 500);
    });

    await page.goto('/crates/nanomsg');
    await expect(page).toHaveURL('/crates/nanomsg');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('nanomsg: Failed to load crate data');
    await expect(page.locator('[data-test-go-back]')).toHaveCount(0);
    await expect(page.locator('[data-test-try-again]')).toBeVisible();
  });

  test('unknown versions fall back to latest version and show an error message', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'nanomsg' });
      server.create('version', { crate, num: '0.6.0' });
      server.create('version', { crate, num: '0.6.1' });
    });

    await page.goto('/crates/nanomsg/0.7.0');

    await expect(page).toHaveURL('/crates/nanomsg/0.7.0');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('nanomsg: Version 0.7.0 not found');
    await expect(page.locator('[data-test-go-back]')).toBeVisible();
    await expect(page.locator('[data-test-try-again]')).toHaveCount(0);
  });

  test('other versions loading error shows an error message', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'nanomsg' });
      server.create('version', { crate, num: '0.6.0' });
      server.create('version', { crate, num: '0.6.1' });

      server.get('/api/v1/crates/:crate_name/versions', {}, 500);
    });

    await page.goto('/');
    await page.click('[data-test-just-updated] [data-test-crate-link="0"]');
    await expect(page).toHaveURL('/crates/nanomsg/0.6.0');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('nanomsg: Failed to load version data');
    await expect(page.locator('[data-test-go-back]')).toHaveCount(0);
    await expect(page.locator('[data-test-try-again]')).toBeVisible();
  });

  test('navigating to the all versions page', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.loadFixtures();
    });

    await page.goto('/crates/nanomsg');
    await page.click('[data-test-versions-tab] a');

    await expect(page.locator('[data-test-page-description]')).toHaveText(
      /All 13\s+versions of nanomsg since\s+December \d+th, 2014/,
    );
  });

  test('navigating to the reverse dependencies page', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.loadFixtures();
    });

    await page.goto('/crates/nanomsg');
    await page.click('[data-test-rev-deps-tab] a');

    await expect(page).toHaveURL('/crates/nanomsg/reverse_dependencies');
    await expect(page.locator('a[href="/crates/unicorn-rpc"]')).toHaveText('unicorn-rpc');
  });

  test('navigating to a user page', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.loadFixtures();
    });

    await page.goto('/crates/nanomsg');
    await page.click('[data-test-owners] [data-test-owner-link="blabaere"]');

    await expect(page).toHaveURL('/users/blabaere');
    await expect(page.locator('[data-test-heading] [data-test-username]')).toHaveText('blabaere');
  });

  test('navigating to a team page', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.loadFixtures();
    });

    await page.goto('/crates/nanomsg');
    await page.click('[data-test-owners] [data-test-owner-link="github:org:thehydroimpulse"]');

    await expect(page).toHaveURL('/teams/github:org:thehydroimpulse');
    await expect(page.locator('[data-test-heading] [data-test-team-name]')).toHaveText('thehydroimpulseteam');
  });

  test('crates having user-owners', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.loadFixtures();
    });

    await page.goto('/crates/nanomsg');

    await expect(
      page.locator('[data-test-owners] [data-test-owner-link="github:org:thehydroimpulse"] img'),
    ).toHaveAttribute('src', 'https://avatars.githubusercontent.com/u/565790?v=3&s=64');

    await expect(page.locator('[data-test-owners] li')).toHaveCount(4);
  });

  test('crates having team-owners', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.loadFixtures();
    });

    await page.goto('/crates/nanomsg');

    await expect(page.locator('[data-test-owners] [data-test-owner-link="github:org:thehydroimpulse"]')).toBeVisible();
    await expect(page.locator('[data-test-owners] li')).toHaveCount(4);
  });

  test('crates license is supplied by version', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.loadFixtures();
    });

    await page.goto('/crates/nanomsg');
    await expect(page.locator('[data-test-license]')).toHaveText('Apache-2.0');

    await page.goto('/crates/nanomsg/0.5.0');
    await expect(page.locator('[data-test-license]')).toHaveText('MIT OR Apache-2.0');
  });

  test.skip('crates can be yanked by owner', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.loadFixtures();

      let user = server.schema.users.findBy({ login: 'thehydroimpulse' });
      authenticateAs(user);
    });

    await page.goto('/crates/nanomsg/0.5.0');
    const yankButton = page.locator('[data-test-version-yank-button="0.5.0"]');
    await yankButton.click();
    await expect(yankButton).toHaveText('Yanking...');
    await expect(yankButton).toBeDisabled();

    const unyankButton = page.locator('[data-test-version-unyank-button="0.5.0"]');
    await unyankButton.click();
    await expect(unyankButton).toHaveText('Unyanking...');
    await expect(unyankButton).toBeDisabled();

    await expect(yankButton).toBeVisible();
  });

  test('navigating to the owners page when not logged in', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.loadFixtures();
    });

    await page.goto('/crates/nanomsg');

    await expect(page.locator('[data-test-settings-tab]')).toHaveCount(0);
  });

  test('navigating to the owners page when not an owner', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.loadFixtures();

      let user = server.schema.users.findBy({ login: 'iain8' });
      authenticateAs(user);
    });

    await page.goto('/crates/nanomsg');

    await expect(page.locator('[data-test-settings-tab]')).toHaveCount(0);
  });

  test('navigating to the settings page', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.loadFixtures();

      let user = server.schema.users.findBy({ login: 'thehydroimpulse' });
      authenticateAs(user);
    });

    await page.goto('/crates/nanomsg');
    await page.click('[data-test-settings-tab] a');

    await expect(page).toHaveURL('/crates/nanomsg/settings');
  });
});
