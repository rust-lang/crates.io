import type { components } from '@crates-io/api-client';
import type { User } from '../models/index.js';

type ApiUser = components['schemas']['User'];
type ApiAuthenticatedUser = components['schemas']['AuthenticatedUser'];
type ApiLinkedAccount = components['schemas']['LinkedAccount'];

export function serializeLinkedAccount(account: User['githubAccounts'][number]): ApiLinkedAccount {
  return {
    provider: 'github',
    account_id: account.accountId,
    login: account.login,
    avatar: account.avatar,
  };
}

export function serializeUser(user: User): ApiUser;
export function serializeUser(user: User, options: { removePrivateData?: true }): ApiUser;
export function serializeUser(user: User, options: { removePrivateData: false }): ApiAuthenticatedUser;
export function serializeUser(
  user: User,
  { removePrivateData = true }: { removePrivateData?: boolean } = {},
): ApiUser | ApiAuthenticatedUser {
  let serialized = {
    id: user.id,
    created_at: null,
    login: user.login,
    name: user.name,
    url: user.url,
    avatar: user.avatar,
  } satisfies Omit<ApiUser, 'github_username_matches'>;

  if (!removePrivateData) {
    return {
      ...serialized,
      email: user.email,
      email_verified: user.emailVerified,
      email_verification_sent: user.emailVerified || Boolean(user.emailVerificationToken),
      is_admin: user.isAdmin,
      publish_notifications: user.publishNotifications,
    };
  }

  return {
    ...serialized,
    github_username_matches: user.githubAccounts.some(account => account.login === user.login),
  };
}
