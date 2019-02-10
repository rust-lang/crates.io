import Response from 'ember-cli-mirage/response';

export default function() {
    // Used by ember-cli-code-coverage
    this.passthrough('/write-coverage');

    this.namespace = '/api/v1';

    this.get('/summary', function(schema) {
        let crates = schema.crates.all();

        let just_updated = crates.sort((a, b) => compareIsoDates(b.updated_at, a.updated_at)).slice(0, 10);
        let most_downloaded = crates.sort((a, b) => b.downloads - a.downloads).slice(0, 10);
        let new_crates = crates.sort((a, b) => compareIsoDates(b.created_at, a.created_at)).slice(0, 10);
        let most_recently_downloaded = crates.sort((a, b) => b.recent_downloads - a.recent_downloads).slice(0, 10);

        let num_crates = crates.length;
        let num_downloads = crates.models.reduce((sum, crate) => sum + crate.downloads, 0);

        let popular_categories = schema.categories
            .all()
            .sort((a, b) => b.crates_cnt - a.crates_cnt)
            .slice(0, 10);
        let popular_keywords = schema.keywords
            .all()
            .sort((a, b) => b.crates_cnt - a.crates_cnt)
            .slice(0, 10);

        return {
            just_updated: this.serialize(just_updated).crates.map(it => ({ ...it, versions: null })),
            most_downloaded: this.serialize(most_downloaded).crates.map(it => ({ ...it, versions: null })),
            new_crates: this.serialize(new_crates).crates.map(it => ({ ...it, versions: null })),
            most_recently_downloaded: this.serialize(most_recently_downloaded).crates.map(it => ({
                ...it,
                versions: null,
            })),
            num_crates,
            num_downloads,
            popular_categories: this.serialize(popular_categories).categories,
            popular_keywords: this.serialize(popular_keywords).keywords,
        };
    });

    this.get('/crates', function(schema, request) {
        const { start, end } = pageParams(request);

        let crates = schema.crates.all();

        if (request.queryParams.letter) {
            let letter = request.queryParams.letter.toLowerCase();
            crates = crates.filter(crate => crate.id[0].toLowerCase() === letter);
        }

        if (request.queryParams.q) {
            let q = request.queryParams.q.toLowerCase();
            crates = crates.filter(crate => crate.id.toLowerCase().indexOf(q) !== -1);
        }

        if (request.queryParams.user_id) {
            let userId = parseInt(request.queryParams.user_id, 10);
            crates = crates.filter(crate => (crate._owner_users || []).indexOf(userId) !== -1);
        }

        if (request.queryParams.team_id) {
            let teamId = parseInt(request.queryParams.team_id, 10);
            crates = crates.filter(crate => (crate._owner_teams || []).indexOf(teamId) !== -1);
        }

        if (request.queryParams.sort === 'alpha') {
            crates = crates.sort((a, b) => compareStrings(a.id.toLowerCase(), b.id.toLowerCase()));
        }

        return withMeta(this.serialize(crates.slice(start, end)), { total: crates.length });
    });

    this.get('/crates/:crate_id', function(schema, request) {
        let crateId = request.params.crate_id;
        let crate = schema.crates.find(crateId);
        let categories = schema.categories
            .all()
            .filter(category => (crate.categories || []).indexOf(category.id) !== -1);
        let keywords = schema.keywords.all().filter(keyword => (crate.keywords || []).indexOf(keyword.id) !== -1);
        let versions = schema.versions
            .all()
            .filter(version => (crate.versions || []).indexOf(parseInt(version.id, 10)) !== -1);

        return {
            ...this.serialize(crate),
            ...this.serialize(categories),
            ...this.serialize(keywords),
            ...this.serialize(versions),
        };
    });

    this.get('/crates/:crate_id/following', (/* schema, request */) => {
        // TODO
    });

    this.get('/crates/:crate_id/versions', (schema, request) => {
        let crate = request.params.crate_id;
        return schema.versions.where({ crate }).sort((a, b) => compareIsoDates(b.created_at, a.created_at));
    });

    this.get('/crates/:crate_id/:version_num/authors', (schema, request) => {
        let crate = request.params.crate_id;
        let num = request.params.version_num;
        let version = schema.versions.findBy({ crate, num });
        return { meta: { names: version._authors }, users: [] };
    });

    this.get('/crates/:crate_id/:version_num/dependencies', (schema, request) => {
        let crate = request.params.crate_id;
        let num = request.params.version_num;
        let version_id = schema.versions.findBy({ crate, num }).id;
        return schema.dependencies.where({ version_id });
    });

    this.get('/crates/:crate_id/:version_num/downloads', function(schema, request) {
        let crateId = request.params.crate_id;
        let versionNum = request.params.version_num;
        let versionId = schema.versions.findBy({ crate: crateId, num: versionNum }).id;
        return schema.versionDownloads.where({ version: versionId });
    });

    this.get('/crates/:crate_id/owner_user', function(schema, request) {
        let crateId = request.params.crate_id;
        let crate = schema.crates.find(crateId);
        let users = schema.users.find(crate._owner_users);

        let response = this.serialize(users);

        response.users.forEach(user => {
            user.kind = 'user';
        });

        return response;
    });

    this.get('/crates/:crate_id/owner_team', function(schema, request) {
        let crateId = request.params.crate_id;
        let crate = schema.crates.find(crateId);
        let teams = schema.teams.find(crate._owner_teams);

        let response = this.serialize(teams);

        response.teams.forEach(team => {
            team.kind = 'team';
        });

        return response;
    });

    this.get('/crates/:crate_id/reverse_dependencies', function(schema, request) {
        let { start, end } = pageParams(request);

        let crate = request.params.crate_id;
        let allDependencies = schema.dependencies.where({ crate_id: crate });
        let dependencies = allDependencies.slice(start, end);
        let total = allDependencies.length;

        let versions = schema.versions.find(dependencies.models.map(it => it.version_id));

        return {
            ...this.serialize(dependencies),
            ...this.serialize(versions),
            meta: { total },
        };
    });

    this.get('/crates/:crate_id/downloads', function(schema, request) {
        let crateId = request.params.crate_id;
        let crate = schema.crates.find(crateId);
        let versionDownloads = schema.versionDownloads
            .all()
            .filter(it => crate.versions.indexOf(parseInt(it.version, 10)) !== -1);

        return withMeta(this.serialize(versionDownloads), { extra_downloads: crate._extra_downloads });
    });

    this.get('/categories', function(schema, request) {
        let { start, end } = pageParams(request);

        let allCategories = schema.categories.all().sort((a, b) => compareStrings(a.category, b.category));
        let categories = allCategories.slice(start, end);
        let total = allCategories.length;

        return withMeta(this.serialize(categories), { total });
    });

    this.get('/categories/:category_id', function(schema, request) {
        let catId = request.params.category_id;
        let category = schema.categories.find(catId);
        return category ? category : notFound();
    });

    this.get('/category_slugs', function(schema) {
        let allCategories = schema.categories.all().sort((a, b) => compareStrings(a.category, b.category));
        return {
            category_slugs: this.serialize(allCategories).categories.map(cat => ({
                id: cat.id,
                slug: cat.slug,
                description: cat.description,
            })),
        };
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

    this.put('/crates/:crate_id/owners', (schema, request) => {
        const crateId = request.params.crate_id;
        const crate = schema.crates.find(crateId);

        if (!crate) {
            return notFound();
        }

        const body = JSON.parse(request.requestBody);
        const [ownerId] = body.owners;
        const user = schema.users.findBy({ login: ownerId });

        if (!user) {
            return notFound();
        }

        return { ok: true };
    });

    this.delete('/crates/:crate_id/owners', (schema, request) => {
        const crateId = request.params.crate_id;
        const crate = schema.crates.find(crateId);

        if (!crate) {
            return notFound();
        }

        const body = JSON.parse(request.requestBody);
        const [ownerId] = body.owners;
        const user = schema.users.findBy({ login: ownerId });

        if (!user) {
            return notFound();
        }

        return {};
    });
}

function notFound() {
    return new Response(
        404,
        { 'Content-Type': 'application/json' },
        {
            errors: [{ detail: 'Not Found' }],
        },
    );
}

function pageParams(request) {
    const { queryParams } = request;

    const page = parseInt(queryParams.page || '1');
    const perPage = parseInt(queryParams.per_page || '10');

    const start = (page - 1) * perPage;
    const end = start + perPage;

    return { page, perPage, start, end };
}

function withMeta(response, meta) {
    response.meta = meta;
    return response;
}

function compareStrings(a, b) {
    return a < b ? -1 : a > b ? 1 : 0;
}

function compareIsoDates(a, b) {
    let aDate = new Date(a);
    let bDate = new Date(b);
    return aDate < bDate ? -1 : aDate > bDate ? 1 : 0;
}
