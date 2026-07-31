import type { SuccessBody } from '../../utils/api-types.js';

import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { getSession } from '../../utils/session.js';

export default http.put<{ user_id: string }>('/api/v1/users/:user_id', async ({ params, request }) => {
  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  if (user.id.toString() !== params.user_id) {
    return HttpResponse.json({ errors: [{ detail: 'current user does not match requested user' }] }, { status: 400 });
  }

  let json = (await request.json()) as {
    user?: { publish_notifications?: boolean | null; email?: string | null };
  } | null;
  if (!json || !json.user) {
    return HttpResponse.json({ errors: [{ detail: 'invalid json request' }] }, { status: 400 });
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
      return HttpResponse.json({ errors: [{ detail: 'empty email rejected' }] }, { status: 400 });
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

  return HttpResponse.json<SuccessBody<'update_user'>>({ ok: true });
});
