import { setupGitHubOAuthRoutes } from '@/e2e/fixtures/github-oauth';
import { AppFixtures, expect, test } from '@/e2e/helper';
import { http } from '@crates-io/msw/utils/openapi-http';
import { Page } from '@playwright/test';

/** Retries loading signup after a mocked request failure. */
async function retrySignupLoad(page: Page, msw: AppFixtures['msw'], message: string) {
  await msw.db.pendingSignup.create({ login: 'ghost' });

  await page.goto('/signup');
  await expect(page.locator('[data-test-title]')).toHaveText(message);

  // Restore the normal response so Try Again can load the pending signup.
  msw.worker.resetHandlers();
  await page.getByRole('button', { name: 'Try Again' }).click();
  await expect(page.getByLabel('Username')).toHaveValue('ghost');
}

test.describe('Acceptance | Signup', { tag: '@acceptance' }, () => {
  test('completes signup and returns to the requested page', async ({ page, msw, a11y }) => {
    await setupGitHubOAuthRoutes(page);

    msw.worker.use(
      http.post('/api/private/session/authorize', async ({ response }) => {
        await msw.db.pendingSignup.create({ login: 'ghost', name: 'Ghost', email: 'ghost@example.com' });
        return response(200).json({ status: 'signup_required' });
      }),
    );

    // We go to `/support` first to test the browser history stack behavior after signup.
    await page.goto('/support');

    // Attempt to load the profile settings page, which requires authentication and shows the error page with a login button.
    await page.goto('/settings/profile?from=signup#profile');

    // GitHub authorization requires account creation, so we are redirected to signup instead of being signed in.
    await page.click('[data-test-login]');
    await expect(page.getByRole('heading', { name: 'Create your crates.io account' })).toBeVisible();
    await expect(page.getByLabel('Username')).toHaveValue('ghost');
    await expect(page.getByLabel('Username')).not.toBeEditable();
    await expect(page.getByLabel('Display name')).toHaveValue('Ghost');
    await expect(page.getByLabel('Display name')).not.toBeEditable();
    await expect(page.getByLabel('Email address')).toHaveValue('ghost@example.com');
    expect(await page.evaluate(() => localStorage.getItem('isLoggedIn'))).toBeNull();
    await a11y.audit();

    // Reload the page to ensure that the form is still populated with the pending signup data.
    await page.reload();
    await expect(page.getByLabel('Username')).toHaveValue('ghost');
    await expect(page.getByLabel('Display name')).toHaveValue('Ghost');
    await expect(page.getByLabel('Email address')).toHaveValue('ghost@example.com');

    // Fill in the email address and agree to the policy, then submit the form.
    await page.getByLabel('Email address').fill('chosen@example.com');
    await page.getByRole('checkbox').check();
    await page.getByRole('button', { name: 'Create account' }).click();

    // After successful signup, we are redirected to the original destination page.
    await expect(page).toHaveURL('/settings/profile?from=signup#profile');
    await expect(page.locator('[data-test-user-menu] [data-test-toggle]')).toHaveText('Ghost');
    expect(msw.db.user.findFirst()?.email).toBe('chosen@example.com');
    expect(msw.db.pendingSignup.findFirst()).toBeFalsy();
    await expect(page.locator('[data-test-notification-message]')).toHaveCount(0);

    // Going back in the browser history should return to the signup page, which now shows a message that the user is already signed in.
    await page.goBack();
    await expect(page).toHaveURL('/signup?returnTo=%2Fsettings%2Fprofile%3Ffrom%3Dsignup%23profile');
    await expect(page.locator('[data-test-title]')).toHaveText('You are already signed in.');

    // Going back again should return to the original destination page, which is now accessible since the user is signed in.
    await page.goBack();
    await expect(page).toHaveURL('/settings/profile?from=signup#profile');

    // Going back again should return to the support page.
    await page.goBack();
    await expect(page).toHaveURL('/support');
  });

  test('requires policy agreement before account creation', async ({ page, msw }) => {
    await msw.db.pendingSignup.create({ login: 'ghost', email: 'ghost@example.com' });
    await page.goto('/signup');

    // Submit without agreeing to the policy to check that the browser blocks account creation.
    let agreement = page.getByRole('checkbox');
    await expect(agreement).not.toBeChecked();
    await page.getByRole('button', { name: 'Create account' }).click();
    await expect(agreement).toBeFocused();
    expect(await agreement.evaluate((input: HTMLInputElement) => input.validity.valueMissing)).toBe(true);
    expect(msw.db.user.findFirst()).toBeFalsy();
  });

  test('restarts expired signup with fresh fields and the original destination', async ({ page, msw }) => {
    await setupGitHubOAuthRoutes(page);

    // Start with an existing pending signup, then make the next GitHub authorization return different details.
    await msw.db.pendingSignup.create({ login: 'old', name: 'Old User', email: 'old@example.com' });

    msw.worker.use(
      http.post('/api/private/session/authorize', async ({ response }) => {
        await msw.db.pendingSignup.create({ login: 'fresh', name: 'Fresh User', email: 'fresh@example.com' });
        return response(200).json({ status: 'signup_required' });
      }),
    );

    await page.goto('/support');
    await page.goto('/signup?returnTo=%2Fsupport%3Fsource%3Dsignup%23help');
    await expect(page.getByLabel('Username')).toHaveValue('old');
    await expect(page.getByLabel('Display name')).toHaveValue('Old User');
    await expect(page.getByLabel('Email address')).toHaveValue('old@example.com');

    // Expire the pending signup while the form is open. Reloading `/signup` now shows an error page.
    msw.db.pendingSignup.deleteMany(q => q.where(() => true));
    await page.reload();
    await expect(page.locator('[data-test-title]')).toHaveText(
      'Your signup session is missing or has expired. Please authenticate with GitHub again.',
    );

    // Reauthenticate from that error page to replace the stale fields without losing the return destination.
    await page.click('[data-test-login-button]');
    await expect(page.getByLabel('Username')).toHaveValue('fresh');
    await expect(page.getByLabel('Display name')).toHaveValue('Fresh User');
    await expect(page.getByLabel('Email address')).toHaveValue('fresh@example.com');

    // Cancellation takes us to the `returnTo` destination.
    await page.getByRole('button', { name: 'Cancel' }).click();
    await expect(page).toHaveURL('/support?source=signup#help');

    // Going back in the browser history returns to the signup URL, which now shows a message that the signup session is missing.
    await page.goBack();
    await expect(page).toHaveURL('/signup?returnTo=%2Fsupport%3Fsource%3Dsignup%23help');
    await expect(page.locator('[data-test-title]')).toHaveText(
      'Your signup session is missing or has expired. Please authenticate with GitHub again.',
    );

    // Going back again returns to the support page.
    await page.goBack();
    await expect(page).toHaveURL('/support');
  });

  test('requires an email address', async ({ page, msw }) => {
    await msw.db.pendingSignup.create({ login: 'ghost' });
    await page.goto('/signup');

    // With no email in the "pending signup details", submitting the empty field should fail native form validation.
    let email = page.getByLabel('Email address');
    await expect(email).toHaveValue('');
    await page.getByRole('button', { name: 'Create account' }).click();
    await expect(email).toBeFocused();
    expect(await email.evaluate((input: HTMLInputElement) => input.validity.valueMissing)).toBe(true);
    expect(msw.db.user.findFirst()).toBeFalsy();
  });

  test('retry loading signup form after a server failure', async ({ page, msw }) => {
    // Return a 503 server error for the first load, then `retrySignupLoad()` will reset the handler.
    msw.worker.use(
      http.get('/api/private/session/signup', ({ response }) =>
        response('5XX').json({ errors: [{ detail: 'Temporarily unavailable' }] }, { status: 503 }),
      ),
    );

    await retrySignupLoad(page, msw, 'Temporarily unavailable');
  });

  test('retry loading signup form after a network failure', async ({ page, msw }) => {
    // Return a network error for the first load, then `retrySignupLoad()` will reset the handler.
    msw.worker.use(http.get('/api/private/session/signup', ({ response }) => response.untyped(Response.error())));

    await retrySignupLoad(page, msw, 'Failed to load signup details');
  });

  test('retains entered email after a submission network failure', async ({ page, msw }) => {
    await msw.db.pendingSignup.create({ login: 'ghost' });

    msw.worker.use(http.post('/api/private/session/signup', ({ response }) => response.untyped(Response.error())));

    // Enter an email and submit the form, which fails with a network error.
    // The form should retain the entered email for another attempt.
    await page.goto('/signup');
    await page.getByLabel('Email address').fill('chosen@example.com');
    await page.getByRole('checkbox').check();
    await page.getByRole('button', { name: 'Create account' }).click();
    await expect(page.getByRole('alert')).toHaveText('Could not complete signup. Please try again.');
    await expect(page.getByLabel('Email address')).toHaveValue('chosen@example.com');
  });

  test('shows signup errors in the form alert and retains email', async ({ page, msw }) => {
    await msw.db.pendingSignup.create({ login: 'ghost' });

    msw.worker.use(
      http.post('/api/private/session/signup', ({ response }) =>
        response.untyped(Response.json({ errors: [{ detail: 'Signup validation failed' }] }, { status: 422 })),
      ),
    );

    // Enter an email and submit the form, which fails with a validation error.
    // The form should retain the entered email for another attempt.
    await page.goto('/signup');
    await page.getByLabel('Email address').fill('chosen@example.com');
    await page.getByRole('checkbox').check();
    await page.getByRole('button', { name: 'Create account' }).click();
    await expect(page.getByRole('alert')).toHaveText('Signup validation failed');
    await expect(page.getByLabel('Email address')).toHaveValue('chosen@example.com');
  });

  test('shows the error page when signup expires before submission', async ({ page, msw }) => {
    await msw.db.pendingSignup.create({ login: 'ghost', email: 'ghost@example.com' });

    await page.goto('/signup');
    await expect(page.getByLabel('Email address')).toHaveValue('ghost@example.com');
    await page.getByRole('checkbox').check();

    // Expire the session after the form loaded.
    msw.db.pendingSignup.deleteMany(q => q.where(() => true));

    // Submitting the form after the pending signup has expired should show the error page.
    await page.getByRole('button', { name: 'Create account' }).click();
    await expect(page.locator('[data-test-title]')).toHaveText(
      'Your signup session is missing or has expired. Please authenticate with GitHub again.',
    );
  });

  test('keeps the form when cancellation fails and allows retry', async ({ page, msw }) => {
    await msw.db.pendingSignup.create({ login: 'ghost' });

    msw.worker.use(
      http.delete('/api/private/session/signup', ({ response }) =>
        response('5XX').json({ errors: [{ detail: 'Cancellation failed' }] }, { status: 503 }),
      ),
    );

    await page.goto('/signup');
    await page.getByLabel('Email address').fill('chosen@example.com');

    // A failed `DELETE` should keep the form and its entered email available for another attempt.
    await page.getByRole('button', { name: 'Cancel' }).click();
    await expect(page.getByRole('alert')).toHaveText('Cancellation failed');
    await expect(page.getByLabel('Email address')).toHaveValue('chosen@example.com');

    msw.worker.resetHandlers();
    await page.getByRole('button', { name: 'Cancel' }).click();
    await expect(page).toHaveURL('/');
  });

  test('rejects external return destinations', async ({ page, msw }) => {
    await msw.db.pendingSignup.create({ login: 'ghost' });

    // Cancellation must fall back to the homepage rather than navigate to another origin.
    await page.goto('/signup?returnTo=https%3A%2F%2Fexample.com%2F');
    await page.getByRole('button', { name: 'Cancel' }).click();
    await expect(page).toHaveURL('/');
  });

  test('prevents duplicate submission while account creation is pending', async ({ page, msw }) => {
    await msw.db.pendingSignup.create({ login: 'ghost', email: 'ghost@example.com' });

    // Hold the signup response so the pending controls and request count can be inspected.
    let deferred = Promise.withResolvers<void>();
    msw.worker.use(
      http.post('/api/private/session/signup', async ({ response }) => {
        await deferred.promise;
        return response('5XX').json({ errors: [{ detail: 'Please retry' }] }, { status: 503 });
      }),
    );

    await page.goto('/signup');
    await page.getByRole('checkbox').check();
    await page.getByRole('button', { name: 'Create account' }).click();
    await expect(page.getByRole('button', { name: 'Create account' })).toBeDisabled();
    await expect(page.getByRole('button', { name: 'Cancel' })).toBeDisabled();
    await expect(page.getByRole('status')).toHaveText('Please wait…');

    // Release the request with an error to check that the form becomes usable again.
    deferred.resolve();
    await expect(page.getByRole('alert')).toHaveText('Please retry');
    await expect(page.getByRole('button', { name: 'Create account' })).toBeEnabled();
  });
});
