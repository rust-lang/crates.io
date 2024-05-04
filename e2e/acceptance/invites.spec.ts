import { test, expect } from '@/e2e/helper';

test.describe('Acceptance | /me/pending-invites', { tag: '@acceptance' }, () => {
  test('shows "page requires authentication" error when not logged in', async ({ page }) => {
    await page.goto('/me/pending-invites');
    await expect(page).toHaveURL('/me/pending-invites');
    await expect(page.locator('[data-test-title]')).toHaveText('This page requires authentication');
    await expect(page.locator('[data-test-login]')).toBeVisible();
  });
});

test.describe('Acceptance | /me/pending-invites', { tag: '@acceptance' }, () => {
  test.beforeEach(async ({ mirage }) => {
    await mirage.addHook(server => {
      let inviter = server.create('user', { name: 'janed' });
      let inviter2 = server.create('user', { name: 'wycats' });

      let user = server.create('user');

      let nanomsg = server.create('crate', { name: 'nanomsg' });
      server.create('version', { crate: nanomsg });
      server.create('crate-owner-invitation', {
        crate: nanomsg,
        createdAt: '2016-12-24T12:34:56Z',
        invitee: user,
        inviter,
      });

      let ember = server.create('crate', { name: 'ember-rs' });
      server.create('version', { crate: ember });
      server.create('crate-owner-invitation', {
        crate: ember,
        createdAt: '2020-12-31T12:34:56Z',
        invitee: user,
        inviter: inviter2,
      });

      authenticateAs(user);

      Object.assign(globalThis, { nanomsg, user });
    });
  });

  test('list all pending crate owner invites', async ({ page }) => {
    await page.goto('/me/pending-invites');
    await expect(page).toHaveURL('/me/pending-invites');
    await expect(page.locator('[data-test-invite]')).toHaveCount(2);

    const nanomasg = page.locator('[data-test-invite="nanomsg"]');
    await expect(nanomasg).toBeVisible();
    await expect(nanomasg.locator('[data-test-date]')).toHaveText('11 months ago');
    await expect(nanomasg.locator('[data-test-accept-button]')).toBeVisible();
    await expect(nanomasg.locator('[data-test-decline-button]')).toBeVisible();

    const emberRs = page.locator('[data-test-invite="ember-rs"]');
    await expect(emberRs).toBeVisible();
    await expect(emberRs.locator('[data-test-crate-link]')).toHaveText('ember-rs');
    await expect(emberRs.locator('[data-test-crate-link]')).toHaveAttribute('href', '/crates/ember-rs');
    await expect(emberRs.locator('[data-test-inviter-link]')).toHaveText('wycats');
    await expect(emberRs.locator('[data-test-inviter-link]')).toHaveAttribute('href', '/users/wycats');
    await expect(emberRs.locator('[data-test-date]')).toHaveText('in about 3 years');
    await expect(emberRs.locator('[data-test-accept-button]')).toBeVisible();
    await expect(emberRs.locator('[data-test-decline-button]')).toBeVisible();

    await expect(page.locator('[data-test-error-message]')).toHaveCount(0);
    await expect(page.locator('[data-test-accepted-message]')).toHaveCount(0);
    await expect(page.locator('[data-test-declined-message]')).toHaveCount(0);
  });

  test('shows empty list message', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.schema.crateOwnerInvitations.all().destroy();
    });

    await page.goto('/me/pending-invites');
    await expect(page).toHaveURL('/me/pending-invites');
    await expect(page.locator('[data-test-invite]')).toHaveCount(0);
    await expect(page.locator('[data-test-empty-state]')).toBeVisible();
  });

  test('invites can be declined', async ({ page }) => {
    await page.goto('/me/pending-invites');
    await expect(page).toHaveURL('/me/pending-invites');

    await page.waitForFunction(expect => {
      const { crateOwnerInvitations } = server.schema;
      return crateOwnerInvitations.where({ crateId: nanomsg.id, inviteeId: user.id }).length === expect;
    }, 1);

    await page.waitForFunction(expect => {
      const { crateOwnerships } = server.schema;
      return crateOwnerships.where({ crateId: nanomsg.id, userId: user.id }).length === expect;
    }, 0);

    const nanomasg = page.locator('[data-test-invite="nanomsg"]');
    await nanomasg.locator('[data-test-decline-button]').click();
    await expect(nanomasg.and(page.locator('[data-test-declined-message]'))).toHaveText(
      'Declined. You have not been added as an owner of crate nanomsg.',
    );
    await expect(nanomasg.locator('[data-test-crate-link]')).toHaveCount(0);
    await expect(nanomasg.locator('[data-test-inviter-link]')).toHaveCount(0);

    await expect(page.locator('[data-test-error-message]')).toHaveCount(0);
    await expect(page.locator('[data-test-accepted-message]')).toHaveCount(0);

    await page.waitForFunction(expect => {
      const { crateOwnerInvitations } = server.schema;
      return crateOwnerInvitations.where({ crateId: nanomsg.id, inviteeId: user.id }).length === expect;
    }, 0);

    await page.waitForFunction(expect => {
      const { crateOwnerships } = server.schema;
      return crateOwnerships.where({ crateId: nanomsg.id, userId: user.id }).length === expect;
    }, 0);
  });

  test('error message is shown if decline request fails', async ({ page, mirage }) => {
    await page.goto('/me/pending-invites');
    await expect(page).toHaveURL('/me/pending-invites');

    await page.evaluate(() => {
      server.put('/api/v1/me/crate_owner_invitations/:crate_id', {}, 500);
    });

    await page.click('[data-test-invite="nanomsg"] [data-test-decline-button]');
    await expect(page.locator('[data-test-notification-message="error"]')).toContainText('Error in declining invite');
    await expect(page.locator('[data-test-accepted-message]')).toHaveCount(0);
    await expect(page.locator('[data-test-declined-message]')).toHaveCount(0);
  });

  test('invites can be accepted', async ({ page, percy }) => {
    await page.goto('/me/pending-invites');
    await expect(page).toHaveURL('/me/pending-invites');

    await page.waitForFunction(expect => {
      const { crateOwnerInvitations } = server.schema;
      return crateOwnerInvitations.where({ crateId: nanomsg.id, inviteeId: user.id }).length === expect;
    }, 1);

    await page.waitForFunction(expect => {
      const { crateOwnerships } = server.schema;
      return crateOwnerships.where({ crateId: nanomsg.id, userId: user.id }).length === expect;
    }, 0);

    await page.click('[data-test-invite="nanomsg"] [data-test-accept-button]');
    await expect(page.locator('[data-test-error-message]')).toHaveCount(0);
    await expect(page.locator('[data-test-declined-message]')).toHaveCount(0);
    await expect(page.locator('[data-test-invite="nanomsg"][data-test-accepted-message]')).toHaveText(
      "Success! You've been added as an owner of crate nanomsg.",
    );
    await expect(page.locator('[data-test-invite="nanomsg"] [data-test-crate-link]')).toHaveCount(0);
    await expect(page.locator('[data-test-invite="nanomsg"] [data-test-inviter-link]')).toHaveCount(0);

    await percy.snapshot();

    await page.waitForFunction(expect => {
      const { crateOwnerInvitations } = server.schema;
      return crateOwnerInvitations.where({ crateId: nanomsg.id, inviteeId: user.id }).length === expect;
    }, 0);

    await page.waitForFunction(expect => {
      const { crateOwnerships } = server.schema;
      return crateOwnerships.where({ crateId: nanomsg.id, userId: user.id }).length === expect;
    }, 1);
  });

  test('error message is shown if accept request fails', async ({ page, mirage }) => {
    await page.goto('/me/pending-invites');
    await expect(page).toHaveURL('/me/pending-invites');

    page.evaluate(() => {
      server.put('/api/v1/me/crate_owner_invitations/:crate_id', {}, 500);
    });

    await page.click('[data-test-invite="nanomsg"] [data-test-accept-button]');
    await expect(page.locator('[data-test-notification-message="error"]')).toHaveText('Error in accepting invite');
    await expect(page.locator('[data-test-accepted-message]')).toHaveCount(0);
    await expect(page.locator('[data-test-declined-message]')).toHaveCount(0);
  });

  test('specific error message is shown if accept request fails', async ({ page, mirage }) => {
    let errorMessage =
      'The invitation to become an owner of the demo_crate crate expired. Please reach out to an owner of the crate to request a new invitation.';
    await page.exposeBinding('_errorMessage', () => errorMessage);
    await mirage.addHook(async server => {
      let errorMessage = await globalThis._errorMessage();
      let payload = { errors: [{ detail: errorMessage }] };
      server.put('/api/v1/me/crate_owner_invitations/:crate_id', payload, 410);
    });

    await page.goto('/me/pending-invites');
    await expect(page).toHaveURL('/me/pending-invites');

    await page.click('[data-test-invite="nanomsg"] [data-test-accept-button]');
    await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(
      'Error in accepting invite: ' + errorMessage,
    );
    await expect(page.locator('[data-test-accepted-message]')).toHaveCount(0);
    await expect(page.locator('[data-test-declined-message]')).toHaveCount(0);
  });
});
