import type { components } from '@crates-io/api-client';
import type { PendingSignup } from '../models/index.js';

type ApiSignupDetails = components['schemas']['SignupDetails'];

/** Serializes MSW pending-signup state for the API response. */
export function serializePendingSignup(pendingSignup: PendingSignup): ApiSignupDetails {
  return {
    login: pendingSignup.login,
    email: pendingSignup.email,
  };
}
