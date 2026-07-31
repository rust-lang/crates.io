import { db } from '../../index.js';
import { serializeUser } from '../../serializers/user.js';
import { http } from '../../utils/openapi-http.js';
import { getSession } from '../../utils/session.js';

export default http.get('/api/v1/me', ({ response }) => {
  let { user } = getSession();
  if (!user) {
    return response.untyped(
      Response.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 }),
    );
  }

  let ownerships = db.crateOwnership.findMany(q => q.where(ownership => ownership.user?.id === user.id));

  return response(200).json({
    user: serializeUser(user, { removePrivateData: false }),
    owned_crates: ownerships.map(ownership => ({
      id: ownership.crate.id,
      name: ownership.crate.name,
      email_notifications: ownership.emailNotifications,
    })),
  });
});
