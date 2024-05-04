import { test, expect } from '@/e2e/helper';

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
    await expect(page.locator('[data-test-error-message]')).toHaveText(
      'You may want to visit crates.io/me/pending-invites to try again.',
    );
  });

  test('shows error for expired token', async ({ page, mirage }) => {
    let errorMessage =
      'The invitation to become an owner of the demo_crate crate expired. Please reach out to an owner of the crate to request a new invitation.';
    await page.exposeBinding('_errorMessage', () => errorMessage);
    await mirage.addHook(server => {
      server.put(
        '/api/v1/me/crate_owner_invitations/accept/:token',
        async () => {
          let errorMessage = await globalThis._errorMessage();
          let payload = { errors: [{ detail: errorMessage }] };
          return payload;
        },
        410,
      );
    });

    await page.goto('/accept-invite/secret123');
    await expect(page).toHaveURL('/accept-invite/secret123');
    await expect(page.locator('[data-test-error-message]')).toHaveText(errorMessage);
  });

  test('shows success for known token', async ({ page, mirage, percy }) => {
    await mirage.addHook(server => {
      let inviter = server.create('user');
      let invitee = server.create('user');
      let crate = server.create('crate', { name: 'nanomsg' });
      server.create('version', { crate });
      let invite = server.create('crate-owner-invitation', { crate, invitee, inviter });

      globalThis.invite = invite;
    });

    // NOTE: Because the current implementation only works with the miragejs server running in the
    // browser, we need to navigate to a random page to trigger the server startup and generate a
    // token. This step will not be necessary in production or once we migrate the miragejs server
    // to run in nodejs.
    await page.goto(`/accept-invite/123`);
    const invite = await page.evaluate(() => ({ token: globalThis.invite.token }));

    await page.goto(`/accept-invite/${invite.token}`);
    await expect(page).toHaveURL(`/accept-invite/${invite.token}`);
    await expect(page.locator('[data-test-success-message]')).toHaveText(
      'Visit your dashboard to view all of your crates, or account settings to manage email notification preferences for all of your crates.',
    );

    await percy.snapshot();
  });
});
