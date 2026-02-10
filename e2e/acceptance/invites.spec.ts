import { expect, test } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';

test.describe('Acceptance | /me/pending-invites', { tag: '@acceptance' }, () => {
  async function prepare(msw) {
    let inviter = await msw.db.user.create({ name: 'janed' });
    let inviter2 = await msw.db.user.create({ name: 'wycats' });

    let user = await msw.db.user.create({});

    let nanomsg = await msw.db.crate.create({ name: 'nanomsg' });
    await msw.db.version.create({ crate: nanomsg });
    await msw.db.crateOwnerInvitation.create({
      crate: nanomsg,
      createdAt: '2016-12-24T12:34:56Z',
      invitee: user,
      inviter,
    });

    let ember = await msw.db.crate.create({ name: 'ember-rs' });
    await msw.db.version.create({ crate: ember });
    await msw.db.crateOwnerInvitation.create({
      crate: ember,
      createdAt: '2020-12-31T12:34:56Z',
      invitee: user,
      inviter: inviter2,
    });

    await msw.authenticateAs(user);

    return { nanomsg, user };
  }

  test('shows "page requires authentication" error when not logged in', async ({ page }) => {
    await page.goto('/me/pending-invites');
    await expect(page).toHaveURL('/me/pending-invites');
    await expect(page.locator('[data-test-title]')).toHaveText('This page requires authentication');
    await expect(page.locator('[data-test-login]')).toBeVisible();
  });

  test('list all pending crate owner invites', async ({ page, msw }) => {
    await prepare(msw);

    await page.goto('/me/pending-invites');
    await expect(page).toHaveURL('/me/pending-invites');
    await expect(page.locator('[data-test-invite]')).toHaveCount(2);

    const nanomsg = page.locator('[data-test-invite="nanomsg"]');
    await expect(nanomsg).toBeVisible();
    await expect(nanomsg.locator('[data-test-date]')).toHaveText('11 months ago');
    await expect(nanomsg.locator('[data-test-accept-button]')).toBeVisible();
    await expect(nanomsg.locator('[data-test-decline-button]')).toBeVisible();

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

  test('shows empty list message', async ({ page, msw }) => {
    await prepare(msw);
    msw.db.crateOwnerInvitation.deleteMany(null);

    await page.goto('/me/pending-invites');
    await expect(page).toHaveURL('/me/pending-invites');
    await expect(page.locator('[data-test-invite]')).toHaveCount(0);
    await expect(page.locator('[data-test-empty-state]')).toBeVisible();
  });

  test('invites can be declined', async ({ page, msw }) => {
    let { nanomsg, user } = await prepare(msw);

    let invites = msw.db.crateOwnerInvitation.findMany(q =>
      q.where(inv => inv.crate.id === nanomsg.id && inv.invitee.id === user.id),
    );
    expect(invites.length).toBe(1);

    let owners = msw.db.crateOwnership.findMany(q =>
      q.where(ownership => ownership.crate.id === nanomsg.id && ownership.user.id === user.id),
    );
    expect(owners.length).toBe(0);

    await page.goto('/me/pending-invites');
    await expect(page).toHaveURL('/me/pending-invites');

    const nanomsgL = page.locator('[data-test-invite="nanomsg"]');
    await nanomsgL.locator('[data-test-decline-button]').click();
    await expect(nanomsgL.and(page.locator('[data-test-declined-message]'))).toHaveText(
      'Declined. You have not been added as an owner of crate nanomsg.',
    );
    await expect(nanomsgL.locator('[data-test-crate-link]')).toHaveCount(0);
    await expect(nanomsgL.locator('[data-test-inviter-link]')).toHaveCount(0);

    await expect(page.locator('[data-test-error-message]')).toHaveCount(0);
    await expect(page.locator('[data-test-accepted-message]')).toHaveCount(0);

    invites = msw.db.crateOwnerInvitation.findMany(q =>
      q.where(inv => inv.crate.id === nanomsg.id && inv.invitee.id === user.id),
    );
    expect(invites.length).toBe(0);

    owners = msw.db.crateOwnership.findMany(q =>
      q.where(ownership => ownership.crate.id === nanomsg.id && ownership.user.id === user.id),
    );
    expect(owners.length).toBe(0);
  });

  test('error message is shown if decline request fails', async ({ page, msw }) => {
    await prepare(msw);

    await page.goto('/me/pending-invites');
    await expect(page).toHaveURL('/me/pending-invites');

    let error = HttpResponse.json({}, { status: 500 });
    await msw.worker.use(http.put('/api/v1/me/crate_owner_invitations/:crate_id', () => error));

    await page.click('[data-test-invite="nanomsg"] [data-test-decline-button]');
    await expect(page.locator('[data-test-notification-message="error"]')).toContainText('Error in declining invite');
    await expect(page.locator('[data-test-accepted-message]')).toHaveCount(0);
    await expect(page.locator('[data-test-declined-message]')).toHaveCount(0);
  });

  test('invites can be accepted', async ({ page, percy, msw }) => {
    let { nanomsg, user } = await prepare(msw);

    let invites = msw.db.crateOwnerInvitation.findMany(q =>
      q.where(inv => inv.crate.id === nanomsg.id && inv.invitee.id === user.id),
    );
    expect(invites.length).toBe(1);

    let owners = msw.db.crateOwnership.findMany(q =>
      q.where(ownership => ownership.crate.id === nanomsg.id && ownership.user.id === user.id),
    );
    expect(owners.length).toBe(0);

    await page.goto('/me/pending-invites');
    await expect(page).toHaveURL('/me/pending-invites');

    await page.click('[data-test-invite="nanomsg"] [data-test-accept-button]');
    await expect(page.locator('[data-test-error-message]')).toHaveCount(0);
    await expect(page.locator('[data-test-declined-message]')).toHaveCount(0);
    await expect(page.locator('[data-test-invite="nanomsg"][data-test-accepted-message]')).toHaveText(
      "Success! You've been added as an owner of crate nanomsg.",
    );
    await expect(page.locator('[data-test-invite="nanomsg"] [data-test-crate-link]')).toHaveCount(0);
    await expect(page.locator('[data-test-invite="nanomsg"] [data-test-inviter-link]')).toHaveCount(0);

    await percy.snapshot();

    invites = msw.db.crateOwnerInvitation.findMany(q =>
      q.where(inv => inv.crate.id === nanomsg.id && inv.invitee.id === user.id),
    );
    expect(invites.length).toBe(0);

    owners = msw.db.crateOwnership.findMany(q =>
      q.where(ownership => ownership.crate.id === nanomsg.id && ownership.user.id === user.id),
    );
    expect(owners.length).toBe(1);
  });

  test('error message is shown if accept request fails', async ({ page, msw }) => {
    await prepare(msw);

    await page.goto('/me/pending-invites');
    await expect(page).toHaveURL('/me/pending-invites');

    let error = HttpResponse.json({}, { status: 500 });
    await msw.worker.use(http.put('/api/v1/me/crate_owner_invitations/:crate_id', () => error));

    await page.click('[data-test-invite="nanomsg"] [data-test-accept-button]');
    await expect(page.locator('[data-test-notification-message="error"]')).toHaveText('Error in accepting invite');
    await expect(page.locator('[data-test-accepted-message]')).toHaveCount(0);
    await expect(page.locator('[data-test-declined-message]')).toHaveCount(0);
  });

  test('specific error message is shown if accept request fails', async ({ page, msw }) => {
    await prepare(msw);

    let errorMessage =
      'The invitation to become an owner of the demo_crate crate expired. Please reach out to an owner of the crate to request a new invitation.';
    let error = HttpResponse.json({ errors: [{ detail: errorMessage }] }, { status: 410 });
    await msw.worker.use(http.put('/api/v1/me/crate_owner_invitations/:crate_id', () => error));

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
