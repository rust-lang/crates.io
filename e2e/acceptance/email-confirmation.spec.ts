import { expect, test } from '@/e2e/helper';

test.describe('Acceptance | Email Confirmation', { tag: '@acceptance' }, () => {
  test('unauthenticated happy path', async ({ page, msw }) => {
    let user = msw.db.user.create({ emailVerificationToken: 'badc0ffee' });

    await page.goto('/confirm/badc0ffee');
    await expect(user.emailVerified).toBe(false);
    await expect(page).toHaveURL('/');
    await expect(page.locator('[data-test-notification-message="success"]')).toBeVisible();

    user = msw.db.user.findFirst({ where: { id: { equals: user.id } } });
    await expect(user.emailVerified).toBe(true);
  });

  test('authenticated happy path', async ({ page, msw, ember }) => {
    let user = msw.db.user.create({ emailVerificationToken: 'badc0ffee' });

    await msw.authenticateAs(user);

    await page.goto('/confirm/badc0ffee');
    await expect(user.emailVerified).toBe(false);
    await expect(page).toHaveURL('/');
    await expect(page.locator('[data-test-notification-message="success"]')).toBeVisible();

    const emailVerified = await ember.evaluate(owner => {
      const { currentUser } = owner.lookup('service:session');
      return currentUser.email_verified;
    });
    expect(emailVerified).toBe(true);

    user = msw.db.user.findFirst({ where: { id: { equals: user.id } } });
    await expect(user.emailVerified).toBe(true);
  });

  test('error case', async ({ page }) => {
    await page.goto('/confirm/badc0ffee');
    await expect(page).toHaveURL('/');
    await expect(page.locator('[data-test-notification-message]')).toHaveText('Unknown error in email confirmation');
  });
});
