import { test, expect } from '@/e2e/helper';
import { format } from 'date-fns/format';

test.describe('Acceptance | sudo', { tag: '@acceptance' }, () => {
  test.beforeEach(async ({ mirage }) => {
    await mirage.addHook(server => {
      const isAdmin = globalThis.isAdmin;
      const user = server.create('user', {
        login: 'johnnydee',
        name: 'John Doe',
        email: 'john@doe.com',
        avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
        isAdmin,
      });

      const crate = server.create('crate', {
        name: 'foo',
        newest_version: '0.1.0',
      });

      const version = server.create('version', {
        crate,
        num: '0.1.0',
      });

      authenticateAs(user);
    });
  });

  test('non-admin users do not see any controls', async ({ page }) => {
    await page.addInitScript(() => (globalThis.isAdmin = false));

    await page.goto('/crates/foo/versions');

    // Test the various header elements.
    await expect(page.locator('[data-test-wizard-hat]')).toHaveCount(0);
    await expect(page.locator('[data-test-disable-admin-actions]')).toHaveCount(0);
    await expect(page.locator('[data-test-enable-admin-actions]')).toHaveCount(0);

    // Assert that there's no yank button, disabled, enabled, or in any state.
    await expect(page.locator('[data-test-version-yank-button="0.1.0"]')).toHaveCount(0);
  });

  test('admin user is not initially in sudo mode', async ({ page }) => {
    await page.addInitScript(() => (globalThis.isAdmin = true));

    await page.goto('/crates/foo/versions');

    await page.locator('[data-test-user-menu]').getByRole('button').click();

    // Test the various header elements.
    await expect(page.locator('[data-test-wizard-hat]')).toHaveCount(0);
    await expect(page.locator('[data-test-disable-admin-actions]')).toHaveCount(0);
    await expect(page.locator('[data-test-enable-admin-actions]')).toBeVisible();

    // Test that the fieldset is present and disabled.
    await expect(page.locator('[data-test-placeholder-fieldset]')).toBeVisible();
    // NOTE: `toBeDisabled()` is not working as expected because the element is not a form control element.
    // Ref: https://github.com/microsoft/playwright/issues/13583#issuecomment-1101704985
    await expect(page.locator('[data-test-placeholder-fieldset]')).toHaveAttribute('disabled', 'disabled');

    // From the perspective of the actual button, it isn't disabled, even though
    // the fieldset effectively makes it unclickable.
    await expect(page.locator('[data-test-version-yank-button="0.1.0"]')).toBeVisible();
  });

  test('admin user can enter sudo mode', async ({ page }) => {
    await page.addInitScript(() => (globalThis.isAdmin = true));
    await page.exposeFunction('format', ((date, options) => format(date, options)) as typeof format);

    await page.goto('/crates/foo/versions');

    await page.locator('[data-test-user-menu]').getByRole('button').click();
    await page.getByRole('button', { name: 'Enable admin actions' }).click();

    // Test the various header elements.
    await expect(page.locator('[data-test-wizard-hat]')).toBeVisible();
    await expect(page.locator('[data-test-disable-admin-actions]')).toBeVisible();
    await expect(page.locator('[data-test-enable-admin-actions]')).toHaveCount(0);

    // Test that the expiry time is sensible. We'll allow a minute either way in
    // case of slow tests or slightly wonky clocks.
    const disable = page.locator('[data-test-disable-admin-actions] > div');
    const seen = await disable.evaluate(async disable => {
      const untilAbout = Date.now() + 6 * 60 * 60 * 1000;
      let seen = 0;
      for (const ts of [untilAbout - 60 * 1000, untilAbout, untilAbout + 60 * 1000]) {
        const time = await globalThis.format(new Date(ts), 'HH:mm');
        if (disable.textContent.includes(time)) {
          seen += 1;
        }
      }
      return seen;
    });
    expect(seen).toBe(1);

    // Test that the fieldset is not present.
    await expect(page.locator('[data-test-placeholder-fieldset]')).toHaveCount(0);
    await expect(page.locator('[data-test-version-yank-button="0.1.0"]')).toBeVisible();
  });

  test('admin can yank a crate in sudo mode', async ({ page }) => {
    await page.addInitScript(() => (globalThis.isAdmin = true));

    await page.goto('/crates/foo/versions');

    await page.locator('[data-test-user-menu]').getByRole('button').click();
    await page.getByRole('button', { name: 'Enable admin actions' }).click();

    const yankButton = page.locator('[data-test-version-yank-button="0.1.0"]');
    const unyankButton = page.locator('[data-test-version-unyank-button="0.1.0"]');

    await yankButton.click();

    await expect(unyankButton).toBeVisible();
    await unyankButton.click();

    await expect(yankButton).toBeVisible();
  });
});
