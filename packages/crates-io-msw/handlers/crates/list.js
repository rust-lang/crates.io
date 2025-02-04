import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeCrate } from '../../serializers/crate.js';
import { pageParams } from '../../utils/handlers.js';
import { getSession } from '../../utils/session.js';

export default http.get('/api/v1/crates', async ({ request }) => {
  let url = new URL(request.url);

  const { start, end } = pageParams(request);

  let crates = db.crate.findMany({});

  if (url.searchParams.get('following') === '1') {
    let { user } = getSession();
    if (!user) {
      return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
    }

    crates = user.followedCrates;
  }

  let letter = url.searchParams.get('letter');
  if (letter) {
    letter = letter.toLowerCase();
    crates = crates.filter(crate => crate.name[0].toLowerCase() === letter);
  }

  let q = url.searchParams.get('q');
  if (q) {
    q = q.toLowerCase();
    crates = crates.filter(crate => crate.name.toLowerCase().includes(q));
  }

  let userId = url.searchParams.get('user_id');
  if (userId) {
    userId = parseInt(userId, 10);
    crates = crates.filter(crate =>
      db.crateOwnership.findFirst({
        where: {
          crate: { id: { equals: crate.id } },
          user: { id: { equals: userId } },
        },
      }),
    );
  }

  let teamId = url.searchParams.get('team_id');
  if (teamId) {
    teamId = parseInt(teamId, 10);
    crates = crates.filter(crate =>
      db.crateOwnership.findFirst({
        where: {
          crate: { id: { equals: crate.id } },
          team: { id: { equals: teamId } },
        },
      }),
    );
  }

  let ids = url.searchParams.getAll('ids[]');
  if (ids.length !== 0) {
    crates = crates.filter(crate => ids.includes(crate.name));
  }

  let sort = url.searchParams.get('sort');
  if (sort === 'alpha') {
    crates = crates.sort((a, b) => compare(a.name.toLowerCase(), b.name.toLowerCase()));
  } else if (sort === 'recent-downloads') {
    crates = crates.sort((a, b) => b.recent_downloads - a.recent_downloads);
  }

  let total = crates.length;
  crates = crates.slice(start, end);
  crates = crates.map(c => ({ ...serializeCrate(c), exact_match: c.name.toLowerCase() === q }));

  return HttpResponse.json({ crates, meta: { total } });
});

export function compare(a, b) {
  return a < b ? -1 : a > b ? 1 : 0;
}
