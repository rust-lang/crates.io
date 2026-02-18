import { expect, test } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';

test.describe('Acceptance | /accept-invite/:token', { tag: '@acceptance' }, () => {
  test('visiting to /accept-invite shows 404 page', async ({ page }) => {
    await page.goto('/accept-invite');
    await expect(page).toHaveURL('/accept-invite');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('Page not found');
  });

  test('visiting to /accept-invite/ shows 404 page', async ({ page }) => {
    await page.goto('/accept-invite/');
    await expect(page).toHaveURL('/accept-invite/');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('Page not found');
  });

  test('shows error for unknown token', async ({ page }) => {
    await page.goto('/accept-invite/unknown');
    await expect(page).toHaveURL('/accept-invite/unknown');
    await expect(page.locator('[data-test-error-message]')).toHaveText('Not Found');
  });

  test('shows error for expired token', async ({ page, msw }) => {
    let errorMessage =
      'The invitation to become an owner of the demo_crate crate expired. Please reach out to an owner of the crate to request a new invitation.';
    let error = HttpResponse.json({ errors: [{ detail: errorMessage }] }, { status: 410 });
    await msw.worker.use(http.put('/api/v1/me/crate_owner_invitations/accept/:token', () => error));

    await page.goto('/accept-invite/secret123');
    await expect(page).toHaveURL('/accept-invite/secret123');
    await expect(page.locator('[data-test-error-message]')).toHaveText(errorMessage);
  });

  test('shows success for known token', async ({ page, msw, percy }) => {
    let inviter = await msw.db.user.create({});
    let invitee = await msw.db.user.create({});
    let crate = await msw.db.crate.create({ name: 'nanomsg' });
    await msw.db.version.create({ crate });
    let invite = await msw.db.crateOwnerInvitation.create({ crate, invitee, inviter });

    await page.goto(`/accept-invite/${invite.token}`);
    await expect(page).toHaveURL(`/accept-invite/${invite.token}`);
    await expect(page.locator('[data-test-success-message]')).toHaveText(
      'Visit your dashboard to view all of your crates, or account settings to manage email notification preferences for all of your crates.',
    );

    await percy.snapshot();
  });
});
