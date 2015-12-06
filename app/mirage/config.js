import summaryFixture from '../mirage/fixtures/summary';
import searchFixture from '../mirage/fixtures/search';

export default function() {
    this.get('/summary', () => summaryFixture);
    this.get('/api/v1/crates', (db, request) => {
        if (request.queryParams.q) {
            const page = parseInt(request.queryParams.page);
            const perPage = parseInt(request.queryParams.per_page);
            const start = (page - 1) * perPage;
            return {
                crates: searchFixture.crates.slice(start, start + perPage),
                meta: searchFixture.meta,
            };
        }
    });
}
