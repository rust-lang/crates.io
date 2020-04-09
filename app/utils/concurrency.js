import { didCancel } from 'ember-concurrency';

export function ignoreCancellation(error) {
  if (!didCancel(error)) {
    // re-throw the non-cancellation error
    throw error;
  }
}
