import Response from 'ember-cli-mirage/response';

import summaryFixture from '../mirage/fixtures/summary';
import searchFixture from '../mirage/fixtures/search';
import crateFixture from '../mirage/fixtures/crate';
import crateOwnersFixture from '../mirage/fixtures/crate_owners';
import crateTeamsFixture from '../mirage/fixtures/crate_teams';
import crateReverseDependenciesFixture from '../mirage/fixtures/crate_reverse_dependencies';
import crateDependenciesFixture from '../mirage/fixtures/crate_dependencies';
import crateDownloadsFixture from '../mirage/fixtures/crate_downloads';

export default function() {
    this.get('/summary', () => summaryFixture);

    this.namespace = '/api/v1';

    this.get('/crates', (schema, request) => {
        const { start, end } = pageParams(request);
        const payload = {
            crates: searchFixture.crates.slice(start, end),
            meta: searchFixture.meta,
        };

        if (request.queryParams.team_id) {
            let teamId = request.queryParams.team_id;
            payload.user = schema.teams.find(teamId);

        } else if (request.queryParams.user_id) {
            let userId = request.queryParams.user_id;
            payload.user = schema.users.find(userId);
        }

        return payload;
    });

    this.get('/crates/:crate_id', () => crateFixture);

    this.get('/crates/:crate_id/versions', (schema, request) => {
        let crate = request.params.crate_id;
        return schema.versions.where({ crate });
    });

    this.get('/crates/:crate_id/:version_num/authors', (schema, request) => {
        let crate = request.params.crate_id;
        let num = request.params.version_num;
        let version = schema.versions.findBy({ crate, num });
        return { meta: { names: version._authors }, users: [] };
    });

    this.get('/crates/:crate_id/owner_user', () => crateOwnersFixture);
    this.get('/crates/:crate_id/owner_team', () => crateTeamsFixture);
    this.get('/crates/:crate_id/reverse_dependencies', () => crateReverseDependenciesFixture);
    this.get('/crates/:crate_id/:version_num/dependencies', () => crateDependenciesFixture);
    this.get('/crates/:crate_id/downloads', () => crateDownloadsFixture);
    this.get('/crates/:crate_id/:version_num/downloads', () => crateDownloadsFixture);

    this.get('/categories', function(schema, request) {
        let { start, end } = pageParams(request);

        let allCategories = schema.categories.all().sort((a, b) => compareStrings(a.category, b.category));
        let categories = allCategories.slice(start, end);
        let total = allCategories.length;

        return withMeta(this.serialize(categories), { total });
    });

    this.get('/keywords', function(schema, request) {
        let { start, end } = pageParams(request);

        let allKeywords = schema.keywords.all().sort((a, b) => a.crates_cnt - b.crates_cnt);
        let keywords = allKeywords.slice(start, end);
        let total = allKeywords.length;

        return withMeta(this.serialize(keywords), { total });
    });

    this.get('/keywords/:keyword_id', (schema, request) => {
        let keywordId = request.params.keyword_id;
        let keyword = schema.keywords.find(keywordId);
        return keyword ? keyword : notFound();
    });

    this.get('/teams/:team_id', (schema, request) => {
        let login = request.params.team_id;
        let team = schema.teams.findBy({ login });
        return team ? team : notFound();
    });

    this.get('/users/:user_id', (schema, request) => {
        let login = request.params.user_id;
        let user = schema.users.findBy({ login });
        return user ? user : notFound();
    });
}

function notFound() {
    return new Response(404, { 'Content-Type': 'application/json' }, {
        'errors': [{ 'detail': 'Not Found' }]
    });
}

function pageParams(request) {
    const { queryParams } = request;

    const page = parseInt(queryParams.page);
    const perPage = parseInt(queryParams.per_page);

    const start = (page - 1) * perPage;
    const end = start + perPage;

    return { page, perPage, start, end };
}

function withMeta(response, meta) {
    response.meta = meta;
    return response;
}

function compareStrings(a, b) {
    return (a < b) ? -1 : (a > b) ? 1 : 0;
}
