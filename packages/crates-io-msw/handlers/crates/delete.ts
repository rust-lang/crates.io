import { db } from '../../index.js';
import { http } from '../../utils/openapi-http.js';
import { getSession } from '../../utils/session.js';

export default http.delete('/api/v1/crates/{name}', ({ params, response }) => {
  let { user } = getSession();
  if (!user) {
    return response.untyped(
      Response.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 }),
    );
  }

  let crate = db.crate.findFirst(q => q.where({ name: params.name }));
  if (!crate) {
    return response.untyped(
      Response.json({ errors: [{ detail: `crate \`${params.name}\` does not exist` }] }, { status: 404 }),
    );
  }

  db.crate.delete(q => q.where({ id: crate.id }));

  return response(204).empty();
});
