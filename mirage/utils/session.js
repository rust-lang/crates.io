export function getSession(schema) {
  let session = schema.mirageSessions.first();
  if (!session || Date.parse(session.expires) < Date.now()) {
    return {};
  }

  let user = schema.users.find(session.userId);
  return { session, user };
}
