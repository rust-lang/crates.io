import { expect, test } from '@/e2e/helper';
import { format } from 'date-fns/format';

test.describe('Acceptance | sudo', { tag: '@acceptance' }, () => {
  async function prepare(msw, { isAdmin = false } = {}) {
    let user = await msw.db.user.create({
      login: 'johnnydee',
      name: 'John Doe',
      email: 'john@doe.com',
      avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
      isAdmin,
    });

    let crate = await msw.db.crate.create({
      name: 'foo',
      newest_version: '0.1.0',
    });

    let version = await msw.db.version.create({
      crate,
      num: '0.1.0',
    });

    await msw.authenticateAs(user);

    return { user, crate, version };
  }

  test('non-admin users do not see any controls', async ({ page, msw }) => {
    await prepare(msw);

    await page.goto('/crates/foo/versions');

    // Test the various header elements.
    await expect(page.locator('[data-test-wizard-hat]')).toHaveCount(0);
    await expect(page.locator('[data-test-disable-admin-actions]')).toHaveCount(0);
    await expect(page.locator('[data-test-enable-admin-actions]')).toHaveCount(0);

    // Assert that there's no dropdown menu toggle, disabled, enabled, or in any state.
    await expect(page.locator('[data-test-actions-toggle]')).toHaveCount(0);
    // Assert that there's no yank button, disabled, enabled, or in any state.
    await expect(page.locator('[data-test-version-yank-button="0.1.0"]')).toHaveCount(0);
  });

  test('admin user is not initially in sudo mode', async ({ page, msw }) => {
    await prepare(msw, { isAdmin: true });

    await page.goto('/crates/foo/versions');

    await page.locator('[data-test-user-menu]').getByRole('button').click();

    // Test the various header elements.
    await expect(page.locator('[data-test-wizard-hat]')).toHaveCount(0);
    await expect(page.locator('[data-test-disable-admin-actions]')).toHaveCount(0);
    await expect(page.locator('[data-test-enable-admin-actions]')).toBeVisible();

    // Test that the fieldset is present and disabled.
    await expect(page.locator('[data-test-placeholder-fieldset]').first()).toBeVisible();
    await expect(page.locator('[data-test-placeholder-fieldset]').first()).toHaveAttribute('disabled');

    await expect(page.locator('[data-test-actions-toggle]')).toBeDisabled();
    await expect(page.locator('[data-test-version-yank-button="0.1.0"]')).toBeHidden();
  });

  test('admin user can enter sudo mode', async ({ page, msw }) => {
    await prepare(msw, { isAdmin: true });
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
    let disable = page.locator('[data-test-disable-admin-actions] > div');
    let seen = await disable.evaluate(async disable => {
      let untilAbout = Date.now() + 6 * 60 * 60 * 1000;
      let seen = 0;
      for (let ts of [untilAbout - 60 * 1000, untilAbout, untilAbout + 60 * 1000]) {
        let time = await globalThis.format(new Date(ts), 'HH:mm');
        if (disable.textContent.includes(time)) {
          seen += 1;
        }
      }
      return seen;
    });
    expect(seen).toBe(1);

    await page.locator('[data-test-actions-toggle]').click();

    // Test that the fieldset is not present.
    await expect(page.locator('[data-test-placeholder-fieldset]')).toHaveCount(0);
    await expect(page.locator('[data-test-version-yank-button="0.1.0"]')).toBeVisible();
  });

  test('admin can yank a crate in sudo mode', async ({ page, msw }) => {
    let { version } = await prepare(msw, { isAdmin: true });

    await page.goto('/crates/foo/versions');

    await page.locator('[data-test-user-menu]').getByRole('button').click();
    await page.getByRole('button', { name: 'Enable admin actions' }).click();

    await page.locator('[data-test-actions-toggle]').click();

    let yankButton = page.locator('[data-test-version-yank-button="0.1.0"]');
    let unyankButton = page.locator('[data-test-version-unyank-button="0.1.0"]');

    await yankButton.click();

    // Verify backend state after yanking
    await expect.poll(() => msw.db.version.findFirst(q => q.where({ id: version.id }))).toMatchObject({ yanked: true });

    await expect(unyankButton).toBeVisible();
    await unyankButton.click();

    // Verify backend state after unyanking
    await expect
      .poll(() => msw.db.version.findFirst(q => q.where({ id: version.id })))
      .toMatchObject({ yanked: false });

    await expect(yankButton).toBeVisible();
  });
});
