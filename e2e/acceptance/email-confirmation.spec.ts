import { expect, test } from '@/e2e/helper';

test.describe('Acceptance | Email Confirmation', { tag: '@acceptance' }, () => {
  test('unauthenticated happy path', async ({ page, msw }) => {
    let email = msw.db.email.create({ verified: false, token: 'badc0ffee' });
    let user = msw.db.user.create({ emails: [email] });

    await expect(email.verified).toBe(false);
    await page.goto('/confirm/badc0ffee');
    await expect(page).toHaveURL('/');
    await expect(page.locator('[data-test-notification-message="success"]')).toBeVisible();

    user = msw.db.user.findFirst({ where: { id: { equals: user.id } } });
    await expect(user.emails[0].verified).toBe(true);
  });

  test('authenticated happy path', async ({ page, msw, ember }) => {
    let email = msw.db.email.create({ token: 'badc0ffee' });
    let user = msw.db.user.create({ emails: [email] });

    await msw.authenticateAs(user);

    await expect(email.verified).toBe(false);
    await page.goto('/confirm/badc0ffee');
    await expect(page).toHaveURL('/');
    await expect(page.locator('[data-test-notification-message="success"]')).toBeVisible();

    const emailVerified = await ember.evaluate(owner => {
      const { currentUser } = owner.lookup('service:session');
      return currentUser.emails[0].verified;
    });
    expect(emailVerified).toBe(true);

    user = msw.db.user.findFirst({ where: { id: { equals: user.id } } });
    await expect(user.emails[0].verified).toBe(true);
  });

  test('error case', async ({ page }) => {
    await page.goto('/confirm/badc0ffee');
    await expect(page).toHaveURL('/');
    await expect(page.locator('[data-test-notification-message]')).toHaveText('Unknown error in email confirmation');
  });
});
