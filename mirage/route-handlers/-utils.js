import Response from 'ember-cli-mirage/response';

export function notFound() {
  return new Response(
    404,
    { 'Content-Type': 'application/json' },
    {
      errors: [{ detail: 'Not Found' }],
    },
  );
}

export function pageParams(request) {
  const { queryParams } = request;

  const page = parseInt(queryParams.page || '1');
  const perPage = parseInt(queryParams.per_page || '10');

  const start = (page - 1) * perPage;
  const end = start + perPage;

  return { page, perPage, start, end };
}

export function withMeta(response, meta) {
  response.meta = meta;
  return response;
}

export function compareStrings(a, b) {
  return a < b ? -1 : a > b ? 1 : 0;
}

export function compareIsoDates(a, b) {
  let aDate = new Date(a);
  let bDate = new Date(b);
  return aDate < bDate ? -1 : aDate > bDate ? 1 : 0;
}
