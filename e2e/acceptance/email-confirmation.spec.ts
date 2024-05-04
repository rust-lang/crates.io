import { test, expect } from '@/e2e/helper';

test.describe('Acceptance | Email Confirmation', { tag: '@acceptance' }, () => {
  test('unauthenticated happy path', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let user = server.create('user', { emailVerificationToken: 'badc0ffee' });
      globalThis.user = user;
    });

    await page.goto('/confirm/badc0ffee');
    await page.waitForFunction(expect => globalThis.user.emailVerified === expect, false);
    await expect(page).toHaveURL('/');
    await expect(page.locator('[data-test-notification-message="success"]')).toBeVisible();

    await page.evaluate(() => globalThis.user.reload());
    await page.waitForFunction(expect => globalThis.user.emailVerified === expect, true);
  });

  test('authenticated happy path', async ({ page, mirage, ember }) => {
    await mirage.addHook(server => {
      let user = server.create('user', { emailVerificationToken: 'badc0ffee' });

      authenticateAs(user);
      globalThis.user = user;
    });

    await page.goto('/confirm/badc0ffee');
    await page.waitForFunction(expect => globalThis.user.emailVerified === expect, false);
    await expect(page).toHaveURL('/');
    await expect(page.locator('[data-test-notification-message="success"]')).toBeVisible();

    const emailVerified = await ember.evaluate(owner => {
      const { currentUser } = owner.lookup('service:session');
      return currentUser.email_verified;
    });
    expect(emailVerified).toBe(true);

    await page.evaluate(() => globalThis.user.reload());
    await page.waitForFunction(expect => globalThis.user.emailVerified === expect, true);
  });

  test('error case', async ({ page }) => {
    await page.goto('/confirm/badc0ffee');
    await expect(page).toHaveURL('/');
    await expect(page.locator('[data-test-notification-message]')).toHaveText('Unknown error in email confirmation');
  });
});
