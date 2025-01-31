import { expect, test } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';

test.describe('Acceptance | Dashboard', { tag: '@acceptance' }, () => {
  test('shows "page requires authentication" error when not logged in', async ({ page }) => {
    await page.goto('/dashboard');
    await expect(page).toHaveURL('/dashboard');
    await expect(page.locator('[data-test-title]')).toHaveText('This page requires authentication');
    await expect(page.locator('[data-test-login]')).toBeVisible();
  });

  test('shows the dashboard when logged in', async ({ page, msw, percy }) => {
    let user = msw.db.user.create({
      login: 'johnnydee',
      name: 'John Doe',
      email: 'john@doe.com',
      avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
    });

    await msw.authenticateAs(user);

    {
      let crate = msw.db.crate.create({ name: 'rand' });
      msw.db.version.create({ crate, num: '0.5.0' });
      msw.db.version.create({ crate, num: '0.6.0' });
      msw.db.version.create({ crate, num: '0.7.0' });
      msw.db.version.create({ crate, num: '0.7.1' });
      msw.db.version.create({ crate, num: '0.7.2' });
      msw.db.version.create({ crate, num: '0.7.3' });
      msw.db.version.create({ crate, num: '0.8.0' });
      msw.db.version.create({ crate, num: '0.8.1' });
      msw.db.version.create({ crate, num: '0.9.0' });
      msw.db.version.create({ crate, num: '1.0.0' });
      msw.db.version.create({ crate, num: '1.1.0' });
      user = msw.db.user.update({
        where: { id: { equals: user.id } },
        data: { followedCrates: [...user.followedCrates, crate] },
      });
    }

    {
      let crate = msw.db.crate.create({ name: 'nanomsg' });
      msw.db.crateOwnership.create({ crate, user });
      msw.db.version.create({ crate, num: '0.1.0' });
      user = msw.db.user.update({
        where: { id: { equals: user.id } },
        data: { followedCrates: [...user.followedCrates, crate] },
      });
    }

    let response = HttpResponse.json({ total_downloads: 3892 });
    await msw.worker.use(http.get(`/api/v1/users/${user.id}/stats`, () => response));

    await page.goto('/dashboard');
    await expect(page).toHaveURL('/dashboard');
    await expect(page.locator('[data-test-feed-list]')).toBeVisible();
    await percy.snapshot();
  });
});
