import { expect, test } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';

test.describe('Acceptance | Email Change', { tag: '@acceptance' }, () => {
  test('happy path', async ({ page, msw }) => {
    let user = msw.db.user.create({ email: 'old@email.com' });
    await msw.authenticateAs(user);

    await page.goto('/settings/profile');
    await expect(page).toHaveURL('/settings/profile');
    const emailInput = page.locator('[data-test-email-input]');
    await expect(emailInput).toBeVisible();
    await expect(emailInput.locator('[data-test-no-email]')).toHaveCount(0);
    await expect(emailInput.locator('[data-test-email-address]')).toContainText('old@email.com');
    await expect(emailInput.locator('[data-test-verified]')).toBeVisible();
    await expect(emailInput.locator('[data-test-not-verified]')).toHaveCount(0);
    await expect(emailInput.locator('[data-test-verification-sent]')).toHaveCount(0);
    await expect(emailInput.locator('[data-test-resend-button]')).toHaveCount(0);

    await emailInput.locator('[data-test-edit-button]').click();
    await expect(emailInput.locator('[data-test-input]')).toHaveValue('old@email.com');
    await expect(emailInput.locator('[data-test-save-button]')).toBeEnabled();
    await expect(emailInput.locator('[data-test-cancel-button]')).toBeEnabled();

    await emailInput.locator('[data-test-input]').fill('');
    await expect(emailInput.locator('[data-test-input]')).toHaveValue('');
    await expect(emailInput.locator('[data-test-save-button]')).toBeDisabled();

    await emailInput.locator('[data-test-input]').fill('new@email.com');
    await expect(emailInput.locator('[data-test-input]')).toHaveValue('new@email.com');
    await expect(emailInput.locator('[data-test-save-button]')).toBeEnabled();

    await emailInput.locator('[data-test-save-button]').click();
    await expect(emailInput.locator('[data-test-email-address]')).toContainText('new@email.com');
    await expect(emailInput.locator('[data-test-verified]')).toHaveCount(0);
    await expect(emailInput.locator('[data-test-not-verified]')).toBeVisible();
    await expect(emailInput.locator('[data-test-verification-sent]')).toBeVisible();
    await expect(emailInput.locator('[data-test-resend-button]')).toBeEnabled();

    user = msw.db.user.findFirst({ where: { id: { equals: user.id } } });
    await expect(user.email).toBe('new@email.com');
    await expect(user.emailVerified).toBe(false);
    await expect(user.emailVerificationToken).toBeDefined();
  });

  test('happy path with `email: null`', async ({ page, msw }) => {
    let user = msw.db.user.create({ email: undefined });
    await msw.authenticateAs(user);

    await page.goto('/settings/profile');
    await expect(page).toHaveURL('/settings/profile');
    const emailInput = page.locator('[data-test-email-input]');
    await expect(emailInput).toBeVisible();
    await expect(emailInput.locator('[data-test-no-email]')).toBeVisible();
    await expect(emailInput.locator('[data-test-email-address]')).toHaveText('');
    await expect(emailInput.locator('[data-test-not-verified]')).toHaveCount(0);
    await expect(emailInput.locator('[data-test-verification-sent]')).toHaveCount(0);
    await expect(emailInput.locator('[data-test-resend-button]')).toHaveCount(0);

    await emailInput.locator('[data-test-edit-button]').click();
    await expect(emailInput.locator('[data-test-input]')).toHaveValue('');
    await expect(emailInput.locator('[data-test-save-button]')).toBeDisabled();
    await expect(emailInput.locator('[data-test-cancel-button]')).toBeEnabled();

    await emailInput.locator('[data-test-input]').fill('new@email.com');
    await expect(emailInput.locator('[data-test-input]')).toHaveValue('new@email.com');
    await expect(emailInput.locator('[data-test-save-button]')).toBeEnabled();

    await emailInput.locator('[data-test-save-button]').click();
    await expect(emailInput.locator('[data-test-no-email]')).toHaveCount(0);
    await expect(emailInput.locator('[data-test-email-address]')).toContainText('new@email.com');
    await expect(emailInput.locator('[data-test-verified]')).toHaveCount(0);
    await expect(emailInput.locator('[data-test-not-verified]')).toBeVisible();
    await expect(emailInput.locator('[data-test-verification-sent]')).toBeVisible();
    await expect(emailInput.locator('[data-test-resend-button]')).toBeEnabled();

    user = msw.db.user.findFirst({ where: { id: { equals: user.id } } });
    await expect(user.email).toBe('new@email.com');
    await expect(user.emailVerified).toBe(false);
    await expect(user.emailVerificationToken).toBeDefined();
  });

  test('cancel button', async ({ page, msw }) => {
    let user = msw.db.user.create({ email: 'old@email.com' });
    await msw.authenticateAs(user);

    await page.goto('/settings/profile');
    const emailInput = page.locator('[data-test-email-input]');
    await emailInput.locator('[data-test-edit-button]').click();
    await emailInput.locator('[data-test-input]').fill('new@email.com');
    await expect(emailInput.locator('[data-test-invalid-email-warning]')).toHaveCount(0);

    await emailInput.locator('[data-test-cancel-button]').click();
    await expect(emailInput.locator('[data-test-email-address]')).toContainText('old@email.com');
    await expect(emailInput.locator('[data-test-verified]')).toBeVisible();
    await expect(emailInput.locator('[data-test-not-verified]')).toHaveCount(0);
    await expect(emailInput.locator('[data-test-verification-sent]')).toHaveCount(0);

    user = msw.db.user.findFirst({ where: { id: { equals: user.id } } });
    await expect(user.email).toBe('old@email.com');
    await expect(user.emailVerified).toBe(true);
    await expect(user.emailVerificationToken).toBe(null);
  });

  test('server error', async ({ page, msw }) => {
    let user = msw.db.user.create({ email: 'old@email.com' });
    await msw.authenticateAs(user);

    let error = HttpResponse.json({}, { status: 500 });
    await msw.worker.use(http.put('/api/v1/users/:user_id', () => error));

    await page.goto('/settings/profile');
    const emailInput = page.locator('[data-test-email-input]');
    await emailInput.locator('[data-test-edit-button]').click();
    await emailInput.locator('[data-test-input]').fill('new@email.com');

    await emailInput.locator('[data-test-save-button]').click();
    await expect(emailInput.locator('[data-test-input]')).toHaveValue('new@email.com');
    await expect(emailInput.locator('[data-test-email-address]')).toHaveCount(0);
    await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(
      'Error in saving email: An unknown error occurred while saving this email.',
    );

    user = msw.db.user.findFirst({ where: { id: { equals: user.id } } });
    await expect(user.email).toBe('old@email.com');
    await expect(user.emailVerified).toBe(true);
    await expect(user.emailVerificationToken).toBe(null);
  });

  test.describe('Resend button', function () {
    test('happy path', async ({ page, msw }) => {
      let user = msw.db.user.create({ email: 'john@doe.com', emailVerificationToken: 'secret123' });
      await msw.authenticateAs(user);

      await page.goto('/settings/profile');
      await expect(page).toHaveURL('/settings/profile');
      const emailInput = page.locator('[data-test-email-input]');
      await expect(emailInput).toBeVisible();
      await expect(emailInput.locator('[data-test-email-address]')).toContainText('john@doe.com');
      await expect(emailInput.locator('[data-test-verified]')).toHaveCount(0);
      await expect(emailInput.locator('[data-test-not-verified]')).toBeVisible();
      await expect(emailInput.locator('[data-test-verification-sent]')).toBeVisible();
      const button = emailInput.locator('[data-test-resend-button]');
      await expect(button).toBeEnabled();
      await expect(button).toHaveText('Resend');

      await button.click();
      await expect(button).toBeDisabled();
      await expect(button).toHaveText('Sent!');
    });

    test('server error', async ({ page, msw }) => {
      let user = msw.db.user.create({ email: 'john@doe.com', emailVerificationToken: 'secret123' });
      await msw.authenticateAs(user);

      let error = HttpResponse.json({}, { status: 500 });
      await msw.worker.use(http.put('/api/v1/users/:user_id/resend', () => error));

      await page.goto('/settings/profile');
      await expect(page).toHaveURL('/settings/profile');
      const emailInput = page.locator('[data-test-email-input]');
      await expect(emailInput).toBeVisible();
      await expect(emailInput.locator('[data-test-email-address]')).toContainText('john@doe.com');
      await expect(emailInput.locator('[data-test-verified]')).toHaveCount(0);
      await expect(emailInput.locator('[data-test-not-verified]')).toBeVisible();
      await expect(emailInput.locator('[data-test-verification-sent]')).toBeVisible();
      const button = emailInput.locator('[data-test-resend-button]');
      await expect(button).toBeEnabled();
      await expect(button).toHaveText('Resend');

      await button.click();
      await expect(button).toBeEnabled();
      await expect(button).toHaveText('Resend');
      await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(
        'Unknown error in resending message',
      );
    });
  });
});
