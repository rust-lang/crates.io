import summaryFixture from '../mirage/fixtures/summary';
import searchFixture from '../mirage/fixtures/search';
import categoriesFixture from '../mirage/fixtures/categories';

export default function() {
    this.get('/summary', () => summaryFixture);

    this.get('/api/v1/crates', (db, request) => {
        if (request.queryParams.q) {
            const { start, end } = pageParams(request);
            return {
                crates: searchFixture.crates.slice(start, end),
                meta: searchFixture.meta,
            };
        }
    });

    this.get('/api/v1/categories', () => categoriesFixture);
}

function pageParams(request) {
    const { queryParams } = request;

    const page = parseInt(queryParams.page);
    const perPage = parseInt(queryParams.per_page);

    const start = (page - 1) * perPage;
    const end = start + perPage;

    return { page, perPage, start, end };
}
