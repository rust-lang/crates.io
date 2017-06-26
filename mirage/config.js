import summaryFixture from '../mirage/fixtures/summary';
import searchFixture from '../mirage/fixtures/search';
import categoriesFixture from '../mirage/fixtures/categories';
import crateFixture from '../mirage/fixtures/crate';
import crateVersionsFixture from '../mirage/fixtures/crate_versions';
import crateAuthorsFixture from '../mirage/fixtures/crate_authors';
import crateOwnersFixture from '../mirage/fixtures/crate_owners';
import crateTeamsFixture from '../mirage/fixtures/crate_teams';
import crateReverseDependenciesFixture from '../mirage/fixtures/crate_reverse_dependencies';
import crateDependenciesFixture from '../mirage/fixtures/crate_dependencies';
import crateDownloadsFixture from '../mirage/fixtures/crate_downloads';
import keywordFixture from '../mirage/fixtures/keyword';
import teamFixture from '../mirage/fixtures/team';
import userFixture from '../mirage/fixtures/user';

export default function() {
    this.get('/summary', () => summaryFixture);

    this.get('/api/v1/crates', (db, request) => {
        const { start, end } = pageParams(request);
        const payload = {
            crates: searchFixture.crates.slice(start, end),
            meta: searchFixture.meta,
        };

        if (request.queryParams.team_id) {
            payload.team = teamFixture.team;
        } else if (request.queryParams.user_id) {
            payload.user = userFixture.user;
        }

        return payload;
    });

    this.get('/api/v1/categories', () => categoriesFixture);

    this.get('/api/v1/crates/nanomsg', () => crateFixture);
    this.get('/api/v1/crates/nanomsg/versions', () => crateVersionsFixture);
    this.get('/api/v1/crates/nanomsg/:version_num/authors', () => crateAuthorsFixture);
    this.get('/api/v1/crates/nanomsg/owner_user', () => crateOwnersFixture);
    this.get('/api/v1/crates/nanomsg/owner_team', () => crateTeamsFixture);
    this.get('/api/v1/crates/nanomsg/reverse_dependencies', () => crateReverseDependenciesFixture);
    this.get('/api/v1/crates/nanomsg/:version_num/dependencies', () => crateDependenciesFixture);
    this.get('/api/v1/crates/nanomsg/downloads', () => crateDownloadsFixture);
    this.get('/api/v1/crates/nanomsg/:version_num/downloads', () => crateDownloadsFixture);
    this.get('/api/v1/keywords/network', () => keywordFixture);
    this.get('/api/v1/teams/:team_id', () => teamFixture);
    this.get('/api/v1/users/:user_id', () => userFixture);
}

function pageParams(request) {
    const { queryParams } = request;

    const page = parseInt(queryParams.page);
    const perPage = parseInt(queryParams.per_page);

    const start = (page - 1) * perPage;
    const end = start + perPage;

    return { page, perPage, start, end };
}
