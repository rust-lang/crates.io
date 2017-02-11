import summaryFixture from '../mirage/fixtures/summary';
import searchFixture from '../mirage/fixtures/search';
import categoriesFixture from '../mirage/fixtures/categories';
import crateFixture from '../mirage/fixtures/crate';
import crateVersionsFixture from '../mirage/fixtures/crate_versions';
import crateAuthorsFixture from '../mirage/fixtures/crate_authors';
import crateOwnersFixture from '../mirage/fixtures/crate_owners';
import crateReverseDependenciesFixture from '../mirage/fixtures/crate_reverse_dependencies';
import crateDependenciesFixture from '../mirage/fixtures/crate_dependencies';
import crateDownloadsFixture from '../mirage/fixtures/crate_downloads';
import keywordFixture from '../mirage/fixtures/keyword';

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

    this.get('/api/v1/crates/nanomsg', () => crateFixture);
    this.get('/api/v1/crates/nanomsg/versions', () => crateVersionsFixture);
    this.get('/api/v1/crates/nanomsg/0.4.2/authors', () => crateAuthorsFixture);
    this.get('/api/v1/crates/nanomsg/owners', () => crateOwnersFixture);
    this.get('/api/v1/crates/nanomsg/reverse_dependencies', () => crateReverseDependenciesFixture);
    this.get('/api/v1/crates/nanomsg/0.4.2/dependencies', () => crateDependenciesFixture);
    this.get('/api/v1/crates/nanomsg/downloads', () => crateDownloadsFixture);
    this.get('/api/v1/keywords/network', () => keywordFixture);
}

function pageParams(request) {
    const { queryParams } = request;

    const page = parseInt(queryParams.page);
    const perPage = parseInt(queryParams.per_page);

    const start = (page - 1) * perPage;
    const end = start + perPage;

    return { page, perPage, start, end };
}
