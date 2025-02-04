import { db } from '../index.js';

export function getSession() {
  let session = db.mswSession.findFirst({});
  if (!session) {
    return {};
  }

  let user = session.user;

  return { session, user };
}
