import { db } from '../../index.js';
import { http } from '../../utils/openapi-http.js';
import { getSession } from '../../utils/session.js';

export default http.put('/api/v1/users/{user}', async ({ params, request, response }) => {
  let { user } = getSession();
  if (!user) {
    return response.untyped(
      Response.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 }),
    );
  }

  if (user.id.toString() !== params.user) {
    return response.untyped(
      Response.json({ errors: [{ detail: 'current user does not match requested user' }] }, { status: 400 }),
    );
  }

  let json = await request.json();
  if (!json || !json.user) {
    return response.untyped(Response.json({ errors: [{ detail: 'invalid json request' }] }, { status: 400 }));
  }
  let userUpdate = json.user;

  if (userUpdate.publish_notifications != null) {
    let publishNotifications = userUpdate.publish_notifications;
    await db.user.update(q => q.where({ id: user.id }), {
      data(user) {
        user.publishNotifications = publishNotifications;
      },
    });
  }

  if (userUpdate.email != null) {
    if (!userUpdate.email) {
      return response.untyped(Response.json({ errors: [{ detail: 'empty email rejected' }] }, { status: 400 }));
    }

    let email = userUpdate.email;
    await db.user.update(q => q.where({ id: user.id }), {
      data(user) {
        user.email = email;
        user.emailVerified = false;
        user.emailVerificationToken = 'secret123';
      },
    });
  }

  return response(200).json({ ok: true });
});
