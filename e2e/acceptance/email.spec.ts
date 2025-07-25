import { expect, test } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';

test.describe('Acceptance | Email Management', { tag: '@acceptance' }, () => {
  test.describe('Add email', () => {
    test('happy path', async ({ page, msw }) => {
      let user = msw.db.user.create({ emails: [msw.db.email.create({ email: 'old@email.com', verified: true })] });
      await msw.authenticateAs(user);

      await page.goto('/settings/profile');
      await expect(page).toHaveURL('/settings/profile');

      const existingEmail = page.locator('[data-test-email-input]:nth-of-type(1)');
      await expect(existingEmail.locator('[data-test-email-address]')).toContainText('old@email.com');
      await expect(existingEmail.locator('[data-test-verified]')).toBeVisible();
      await expect(existingEmail.locator('[data-test-unverified]')).toHaveCount(0);
      await expect(existingEmail.locator('[data-test-verification-sent]')).toHaveCount(0);
      await expect(existingEmail.locator('[data-test-resend-button]')).toHaveCount(0);
      await expect(existingEmail.locator('[data-test-remove-button]')).toHaveCount(0);

      await expect(page.locator('[data-test-add-email-button]')).toBeVisible();
      await expect(page.locator('[data-test-add-email-input]')).not.toBeVisible();

      await page.locator('[data-test-add-email-button]').click();

      const addEmailForm = page.locator('[data-test-add-email-input]');
      const inputField = addEmailForm.locator('[data-test-input]');
      const submitButton = addEmailForm.locator('[data-test-save-button]');

      await expect(addEmailForm).toBeVisible();
      await expect(addEmailForm.locator('[data-test-no-email]')).toHaveCount(0);
      await expect(addEmailForm.locator('[data-test-unverified]')).toHaveCount(0);
      await expect(addEmailForm.locator('[data-test-verified]')).toHaveCount(0);
      await expect(addEmailForm.locator('[data-test-verification-sent]')).toHaveCount(0);
      await expect(addEmailForm.locator('[data-test-resend-button]')).toHaveCount(0);
      await expect(inputField).toContainText('');
      await expect(submitButton).toBeDisabled();

      await inputField.fill('');
      await expect(inputField).toHaveValue('');
      await expect(submitButton).toBeDisabled();

      await inputField.fill('notanemail');
      await expect(inputField).toHaveValue('notanemail');
      await expect(submitButton).toBeDisabled();

      await inputField.fill('new@email.com');
      await expect(inputField).toHaveValue('new@email.com');
      await expect(submitButton).toBeEnabled();

      await submitButton.click();
      const createdEmail = page.locator('[data-test-email-input]:nth-of-type(2)');
      await expect(createdEmail.locator('[data-test-email-address]')).toContainText('new@email.com');
      await expect(createdEmail.locator('[data-test-verified]')).toHaveCount(0);
      await expect(createdEmail.locator('[data-test-unverified]')).toHaveCount(0);
      await expect(createdEmail.locator('[data-test-verification-sent]')).toBeVisible();
      await expect(createdEmail.locator('[data-test-resend-button]')).toBeEnabled();

      user = msw.db.user.findFirst({ where: { id: { equals: user.id } } });
      await expect(user.emails.length).toBe(2);
      await expect(user.emails[0].email).toBe('old@email.com');
      await expect(user.emails[1].email).toBe('new@email.com');
      await expect(user.emails[1].verified).toBe(false);
    });

    test('happy path with no previous emails', async ({ page, msw }) => {
      let user = msw.db.user.create({ emails: [] });
      await msw.authenticateAs(user);

      await page.goto('/settings/profile');
      await expect(page).toHaveURL('/settings/profile');

      const addEmailButton = page.locator('[data-test-add-email-button]');
      const addEmailForm = page.locator('[data-test-add-email-input]');
      const addEmailInput = addEmailForm.locator('[data-test-input]');

      await expect(page.locator('[data-test-email-input]')).toHaveCount(0);
      await expect(page.locator('[data-test-add-email-input]')).toHaveCount(0);
      await expect(addEmailButton).toBeVisible();

      await addEmailButton.click();
      await expect(addEmailForm).toBeVisible();
      await expect(addEmailForm.locator('[data-test-input]')).toContainText('');
      await addEmailInput.fill('new@email.com');
      await expect(addEmailInput).toHaveValue('new@email.com');
      await addEmailForm.locator('[data-test-save-button]').click();

      const createdEmail = page.locator('[data-test-email-input]:nth-of-type(1)');
      await expect(createdEmail.locator('[data-test-email-address]')).toContainText('new@email.com');
      await expect(createdEmail.locator('[data-test-verified]')).toHaveCount(0);
      await expect(createdEmail.locator('[data-test-unverified]')).toHaveCount(0);
      await expect(createdEmail.locator('[data-test-verification-sent]')).toBeVisible();
      await expect(createdEmail.locator('[data-test-resend-button]')).toBeEnabled();

      user = msw.db.user.findFirst({ where: { id: { equals: user.id } } });
      await expect(user.emails.length).toBe(1);
      await expect(user.emails[0].email).toBe('new@email.com');
      await expect(user.emails[0].verified).toBe(false);
    });

    test('server error', async ({ page, msw }) => {
      let user = msw.db.user.create({ emails: [] });
      await msw.authenticateAs(user);

      let error = HttpResponse.json({}, { status: 500 });
      await msw.worker.use(http.post('/api/v1/users/:user_id/emails', () => error));

      await page.goto('/settings/profile');

      const addEmailForm = page.locator('[data-test-add-email-input]');

      await page.locator('[data-test-add-email-button]').click();
      await addEmailForm.locator('[data-test-input]').fill('new@email.com');
      await addEmailForm.locator('[data-test-save-button]').click();

      await expect(page.locator('[data-test-email-input]')).toHaveCount(0);
      await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(
        'Unknown error in saving email',
      );

      user = msw.db.user.findFirst({ where: { id: { equals: user.id } } });
      await expect(user.emails.length).toBe(0);
    });
  });

  test.describe('Remove email', () => {
    test('happy path', async ({ page, msw }) => {
      let user = msw.db.user.create({
        emails: [msw.db.email.create({ email: 'john@doe.com' }), msw.db.email.create({ email: 'jane@doe.com' })],
      });
      await msw.authenticateAs(user);

      await page.goto('/settings/profile');
      await expect(page).toHaveURL('/settings/profile');
      const emailInputs = page.locator('[data-test-email-input]');

      await expect(emailInputs).toHaveCount(2);
      const firstEmailInput = emailInputs.nth(0);
      const secondEmailInput = emailInputs.nth(1);

      await expect(firstEmailInput.locator('[data-test-email-address]')).toContainText('john@doe.com');
      await expect(secondEmailInput.locator('[data-test-email-address]')).toContainText('jane@doe.com');

      await expect(firstEmailInput.locator('[data-test-remove-button]')).toBeVisible();
      await expect(secondEmailInput.locator('[data-test-remove-button]')).toBeVisible();

      await secondEmailInput.locator('[data-test-remove-button]').click();
      await expect(emailInputs).toHaveCount(1);
      await expect(firstEmailInput.locator('[data-test-remove-button]')).toHaveCount(0);

      user = msw.db.user.findFirst({ where: { id: { equals: user.id } } });
      await expect(user.emails.length).toBe(1);
      await expect(user.emails[0].email).toBe('john@doe.com');
    });

    test('cannot remove notifications email', async ({ page, msw }) => {
      let user = msw.db.user.create({
        emails: [
          msw.db.email.create({ email: 'notifications@doe.com', send_notifications: true }),
          msw.db.email.create({ email: 'john@doe.com' }),
        ],
      });
      await msw.authenticateAs(user);

      await page.goto('/settings/profile');
      await expect(page).toHaveURL('/settings/profile');

      const emailInputs = page.locator('[data-test-email-input]');
      await expect(emailInputs).toHaveCount(2);
      const notificationsEmailInput = emailInputs.nth(0);
      const johnEmailInput = emailInputs.nth(1);

      await expect(notificationsEmailInput.locator('[data-test-email-address]')).toContainText('notifications@doe.com');
      await expect(johnEmailInput.locator('[data-test-email-address]')).toContainText('john@doe.com');

      await expect(notificationsEmailInput.locator('[data-test-remove-button]')).toBeDisabled();
      await expect(notificationsEmailInput.locator('[data-test-remove-button]')).toHaveAttribute(
        'title',
        'Cannot delete notifications email',
      );
      await expect(johnEmailInput.locator('[data-test-remove-button]')).toBeVisible();
    });

    test('no delete button when only one email', async ({ page, msw }) => {
      let user = msw.db.user.create({
        emails: [
          msw.db.email.create({
            email: 'john@doe.com',
          }),
        ],
      });
      await msw.authenticateAs(user);

      await page.goto('/settings/profile');
      await expect(page).toHaveURL('/settings/profile');
      const emailInput = page.locator('[data-test-email-input]');
      await expect(emailInput).toBeVisible();
      await expect(emailInput.locator('[data-test-email-address]')).toContainText('john@doe.com');
      await expect(emailInput.locator('[data-test-remove-button]')).toHaveCount(0);
    });

    test('server error', async ({ page, msw }) => {
      let user = msw.db.user.create({
        emails: [msw.db.email.create({ email: 'john@doe.com' }), msw.db.email.create({ email: 'jane@doe.com' })],
      });
      await msw.authenticateAs(user);

      let error = HttpResponse.json({}, { status: 500 });
      await msw.worker.use(http.delete('/api/v1/users/:user_id/emails/:email_id', () => error));

      await page.goto('/settings/profile');

      const emailInputs = page.locator('[data-test-email-input]');
      await expect(emailInputs).toHaveCount(2);
      const johnEmailInput = emailInputs.nth(0);
      await expect(johnEmailInput.locator('[data-test-email-address]')).toContainText('john@doe.com');
      await expect(johnEmailInput.locator('[data-test-remove-button]')).toBeEnabled();
      await johnEmailInput.locator('[data-test-remove-button]').click();
      await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(
        'Unknown error in deleting email',
      );
      await expect(johnEmailInput.locator('[data-test-remove-button]')).toBeEnabled();
    });
  });

  test.describe('Resend verification email', function () {
    test('happy path', async ({ page, msw }) => {
      let user = msw.db.user.create({
        emails: [msw.db.email.create({ email: 'john@doe.com', verified: false, verification_email_sent: true })],
      });
      await msw.authenticateAs(user);

      await page.goto('/settings/profile');

      const emailInput = page.locator('[data-test-email-input]:nth-of-type(1)');
      await expect(emailInput.locator('[data-test-email-address]')).toContainText('john@doe.com');

      const resendButton = emailInput.locator('[data-test-resend-button]');
      await expect(resendButton).toBeEnabled();
      await expect(resendButton).toContainText('Resend');
      await expect(emailInput.locator('[data-test-verified]')).toHaveCount(0);
      await expect(emailInput.locator('[data-test-unverified]')).toHaveCount(0);
      await expect(emailInput.locator('[data-test-verification-sent]')).toBeVisible();

      await resendButton.click();
      await expect(emailInput.locator('[data-test-verification-sent]')).toBeVisible();
      await expect(resendButton).toContainText('Sent!');
      await expect(resendButton).toBeDisabled();
    });

    test('server error', async ({ page, msw }) => {
      let user = msw.db.user.create({
        emails: [msw.db.email.create({ email: 'john@doe.com', verified: false, verification_email_sent: true })],
      });
      await msw.authenticateAs(user);

      let error = HttpResponse.json({}, { status: 500 });
      await msw.worker.use(http.put('/api/v1/users/:user_id/emails/:email_id/resend', () => error));

      await page.goto('/settings/profile');

      const emailInput = page.locator('[data-test-email-input]:nth-of-type(1)');
      await expect(emailInput.locator('[data-test-email-address]')).toContainText('john@doe.com');

      const resendButton = emailInput.locator('[data-test-resend-button]');
      await expect(resendButton).toBeEnabled();

      await resendButton.click();
      await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(
        'Unknown error in resending message',
      );
      await expect(resendButton).toBeEnabled();
    });
  });

  test.describe('Switch notification email', () => {
    test('happy path', async ({ page, msw }) => {
      let user = msw.db.user.create({
        emails: [
          msw.db.email.create({ email: 'john@doe.com', verified: true, send_notifications: true }),
          msw.db.email.create({ email: 'jane@doe.com', verified: true, send_notifications: false }),
        ],
      });
      await msw.authenticateAs(user);

      await page.goto('/settings/profile');

      const emailInputs = page.locator('[data-test-email-input]');
      await expect(emailInputs).toHaveCount(2);

      const johnEmailInput = emailInputs.nth(0);
      const janeEmailInput = emailInputs.nth(1);

      await expect(johnEmailInput.locator('[data-test-email-address]')).toContainText('john@doe.com');
      await expect(janeEmailInput.locator('[data-test-email-address]')).toContainText('jane@doe.com');

      const johnEnableNotificationsButton = johnEmailInput.locator('[data-test-notification-button]');
      const janeEnableNotificationsButton = janeEmailInput.locator('[data-test-notification-button]');

      await expect(johnEmailInput.locator('[data-test-notification-target]')).toBeVisible();
      await expect(janeEmailInput.locator('[data-test-notification-target]')).toHaveCount(0);
      await expect(johnEnableNotificationsButton).toHaveCount(0);
      await expect(janeEnableNotificationsButton).toBeEnabled();

      await janeEnableNotificationsButton.click();
      await expect(johnEmailInput.locator('[data-test-notification-target]')).toHaveCount(0);
      await expect(janeEmailInput.locator('[data-test-notification-target]')).toBeVisible();
      await expect(johnEnableNotificationsButton).toBeEnabled();
      await expect(janeEnableNotificationsButton).toHaveCount(0);
    });
  });
});
