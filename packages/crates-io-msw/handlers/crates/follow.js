import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { notFound } from '../../utils/handlers.js';
import { getSession } from '../../utils/session.js';

export default http.put('/api/v1/crates/:name/follow', async ({ params }) => {
  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let crate = db.crate.findFirst(q => q.where({ name: params.name }));
  if (!crate) {
    return notFound();
  }

  await db.user.update(q => q.where({ id: user.id }), {
    data(user) {
      user.followedCrates = [...user.followedCrates.filter(c => c.id !== crate.id), crate];
    },
  });

  return HttpResponse.json({ ok: true });
});
