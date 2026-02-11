import type { components } from '@crates-io/api-client';

import { http, HttpResponse } from 'msw';
import { describe, expect } from 'vitest';
import { render } from 'vitest-browser-svelte';
import { page, userEvent } from 'vitest/browser';

import { test } from '../../test/msw';
import EmailInputTestWrapper from './EmailInputTestWrapper.svelte';

type AuthenticatedUser = components['schemas']['AuthenticatedUser'];

function createUser(overrides: Partial<AuthenticatedUser> = {}): AuthenticatedUser {
  return {
    id: 42,
    login: 'johndoe',
    name: 'John Doe',
    avatar: 'https://avatars.githubusercontent.com/u/1234567?v=4',
    url: 'https://github.com/johndoe',
    is_admin: false,
    publish_notifications: true,
    email: 'old@email.com',
    email_verified: true,
    email_verification_sent: true,
    ...overrides,
  };
}

test('happy path', async ({ worker }) => {
  let user = createUser();
  render(EmailInputTestWrapper, { user });

  worker.use(http.put('/api/v1/users/:user_id', () => HttpResponse.json({ ok: true })));

  await expect.element(page.getByCSS('[data-test-email-input]')).toBeVisible();
  await expect.element(page.getByCSS('[data-test-no-email]')).not.toBeInTheDocument();
  await expect.element(page.getByCSS('[data-test-email-address]')).toHaveTextContent('old@email.com');
  await expect.element(page.getByCSS('[data-test-verified]')).toBeVisible();
  await expect.element(page.getByCSS('[data-test-not-verified]')).not.toBeInTheDocument();
  await expect.element(page.getByCSS('[data-test-verification-sent]')).not.toBeInTheDocument();
  await expect.element(page.getByCSS('[data-test-resend-button]')).not.toBeInTheDocument();

  await page.getByCSS('[data-test-edit-button]').click();
  await expect.element(page.getByCSS('[data-test-input]')).toHaveValue('old@email.com');
  await expect.element(page.getByCSS('[data-test-save-button]')).toBeEnabled();
  await expect.element(page.getByCSS('[data-test-cancel-button]')).toBeEnabled();

  await userEvent.clear(page.getByCSS('[data-test-input]'));
  await expect.element(page.getByCSS('[data-test-input]')).toHaveValue('');
  await expect.element(page.getByCSS('[data-test-save-button]')).toBeDisabled();

  await userEvent.fill(page.getByCSS('[data-test-input]'), 'new@email.com');
  await expect.element(page.getByCSS('[data-test-input]')).toHaveValue('new@email.com');
  await expect.element(page.getByCSS('[data-test-save-button]')).toBeEnabled();

  await page.getByCSS('[data-test-save-button]').click();
  await expect.element(page.getByCSS('[data-test-email-address]')).toHaveTextContent('new@email.com');
  await expect.element(page.getByCSS('[data-test-verified]')).not.toBeInTheDocument();
  await expect.element(page.getByCSS('[data-test-not-verified]')).toBeVisible();
  await expect.element(page.getByCSS('[data-test-verification-sent]')).toBeVisible();
  await expect.element(page.getByCSS('[data-test-resend-button]')).toBeEnabled();
});

test('happy path with `email: null`', async ({ worker }) => {
  let user = createUser({ email: null, email_verified: false, email_verification_sent: false });
  render(EmailInputTestWrapper, { user });

  worker.use(http.put('/api/v1/users/:user_id', () => HttpResponse.json({ ok: true })));

  await expect.element(page.getByCSS('[data-test-email-input]')).toBeVisible();
  await expect.element(page.getByCSS('[data-test-no-email]')).toBeVisible();
  await expect.element(page.getByCSS('[data-test-email-address]')).toHaveTextContent('');
  await expect.element(page.getByCSS('[data-test-not-verified]')).not.toBeInTheDocument();
  await expect.element(page.getByCSS('[data-test-verification-sent]')).not.toBeInTheDocument();
  await expect.element(page.getByCSS('[data-test-resend-button]')).not.toBeInTheDocument();

  await page.getByCSS('[data-test-edit-button]').click();
  await expect.element(page.getByCSS('[data-test-input]')).toHaveValue('');
  await expect.element(page.getByCSS('[data-test-save-button]')).toBeDisabled();
  await expect.element(page.getByCSS('[data-test-cancel-button]')).toBeEnabled();

  await userEvent.fill(page.getByCSS('[data-test-input]'), 'new@email.com');
  await expect.element(page.getByCSS('[data-test-input]')).toHaveValue('new@email.com');
  await expect.element(page.getByCSS('[data-test-save-button]')).toBeEnabled();

  await page.getByCSS('[data-test-save-button]').click();
  await expect.element(page.getByCSS('[data-test-no-email]')).not.toBeInTheDocument();
  await expect.element(page.getByCSS('[data-test-email-address]')).toHaveTextContent('new@email.com');
  await expect.element(page.getByCSS('[data-test-verified]')).not.toBeInTheDocument();
  await expect.element(page.getByCSS('[data-test-not-verified]')).toBeVisible();
  await expect.element(page.getByCSS('[data-test-verification-sent]')).toBeVisible();
  await expect.element(page.getByCSS('[data-test-resend-button]')).toBeEnabled();
});

test('cancel button', async () => {
  let user = createUser();
  render(EmailInputTestWrapper, { user });

  await page.getByCSS('[data-test-edit-button]').click();
  await userEvent.fill(page.getByCSS('[data-test-input]'), 'new@email.com');

  await page.getByCSS('[data-test-cancel-button]').click();
  await expect.element(page.getByCSS('[data-test-email-address]')).toHaveTextContent('old@email.com');
  await expect.element(page.getByCSS('[data-test-verified]')).toBeVisible();
  await expect.element(page.getByCSS('[data-test-not-verified]')).not.toBeInTheDocument();
  await expect.element(page.getByCSS('[data-test-verification-sent]')).not.toBeInTheDocument();
});

test('server error', async ({ worker }) => {
  let user = createUser();
  render(EmailInputTestWrapper, { user });

  worker.use(http.put('/api/v1/users/:user_id', () => HttpResponse.json({}, { status: 500 })));

  await page.getByCSS('[data-test-edit-button]').click();
  await userEvent.fill(page.getByCSS('[data-test-input]'), 'new@email.com');

  await page.getByCSS('[data-test-save-button]').click();
  await expect.element(page.getByCSS('[data-test-input]')).toHaveValue('new@email.com');
  await expect.element(page.getByCSS('[data-test-email-address]')).not.toBeInTheDocument();
  await expect
    .element(page.getByCSS('[data-test-notification-message="error"]'))
    .toHaveTextContent('Error in saving email: An unknown error occurred while saving this email.');
});

describe('Resend button', () => {
  test('happy path', async ({ worker }) => {
    let user = createUser({ email_verified: false, email_verification_sent: true });
    render(EmailInputTestWrapper, { user });

    worker.use(http.put('/api/v1/users/:user_id/resend', () => HttpResponse.json({ ok: true })));

    await expect.element(page.getByCSS('[data-test-email-input]')).toBeVisible();
    await expect.element(page.getByCSS('[data-test-email-address]')).toHaveTextContent('old@email.com');
    await expect.element(page.getByCSS('[data-test-verified]')).not.toBeInTheDocument();
    await expect.element(page.getByCSS('[data-test-not-verified]')).toBeVisible();
    await expect.element(page.getByCSS('[data-test-verification-sent]')).toBeVisible();
    await expect.element(page.getByCSS('[data-test-resend-button]')).toBeEnabled();
    await expect.element(page.getByCSS('[data-test-resend-button]')).toHaveTextContent('Resend');

    await page.getByCSS('[data-test-resend-button]').click();
    await expect.element(page.getByCSS('[data-test-resend-button]')).toBeDisabled();
    await expect.element(page.getByCSS('[data-test-resend-button]')).toHaveTextContent('Sent!');
  });

  test('server error', async ({ worker }) => {
    let user = createUser({ email_verified: false, email_verification_sent: true });
    render(EmailInputTestWrapper, { user });

    worker.use(http.put('/api/v1/users/:user_id/resend', () => HttpResponse.json({}, { status: 500 })));

    await expect.element(page.getByCSS('[data-test-email-input]')).toBeVisible();
    await expect.element(page.getByCSS('[data-test-email-address]')).toHaveTextContent('old@email.com');
    await expect.element(page.getByCSS('[data-test-verified]')).not.toBeInTheDocument();
    await expect.element(page.getByCSS('[data-test-not-verified]')).toBeVisible();
    await expect.element(page.getByCSS('[data-test-verification-sent]')).toBeVisible();
    await expect.element(page.getByCSS('[data-test-resend-button]')).toBeEnabled();
    await expect.element(page.getByCSS('[data-test-resend-button]')).toHaveTextContent('Resend');

    await page.getByCSS('[data-test-resend-button]').click();
    await expect.element(page.getByCSS('[data-test-resend-button]')).toBeEnabled();
    await expect.element(page.getByCSS('[data-test-resend-button]')).toHaveTextContent('Resend');
    await expect
      .element(page.getByCSS('[data-test-notification-message="error"]'))
      .toHaveTextContent('Unknown error in resending message');
  });
});
