import { module, test } from 'qunit';
import { setupApplicationTest } from 'ember-qunit';
import { currentURL } from '@ember/test-helpers';
import { percySnapshot } from 'ember-percy';

import setupMirage from '../helpers/setup-mirage';
import { visit } from '../helpers/visit-ignoring-abort';

module('Acceptance | Dashboard', function (hooks) {
  setupApplicationTest(hooks);
  setupMirage(hooks);

  test('redirects to / when not logged in', async function (assert) {
    await visit('/dashboard');
    assert.equal(currentURL(), '/');
    assert.dom('[data-test-flash-message]').hasText('Please log in to proceed');
  });

  test('shows the dashboard when logged in', async function (assert) {
    let user = this.server.create('user', {
      login: 'johnnydee',
      name: 'John Doe',
      email: 'john@doe.com',
      avatar: 'https://avatars2.githubusercontent.com/u/1234567?v=4',
    });

    this.authenticateAs(user);

    {
      let crate = this.server.create('crate', { name: 'rand' });
      this.server.create('version', { crate, num: '1.0.0' });
      this.server.create('version', { crate, num: '1.1.0' });
    }

    {
      let crate = this.server.create('crate', { name: 'nanomsg' });
      this.server.create('crate-ownership', { crate, user });
      this.server.create('version', { crate, num: '0.1.0' });
    }

    this.server.get('/api/v1/me/updates', {
      versions: [
        {
          id: 152946,
          crate: 'geo',
          num: '0.12.2',
          dl_path: '/api/v1/crates/geo/0.12.2/download',
          readme_path: '/api/v1/crates/geo/0.12.2/readme',
          updated_at: '2019-05-26T14:47:10.220868+00:00',
          created_at: '2019-05-26T14:47:10.220868+00:00',
          downloads: 19372,
          features: {
            default: [],
            'postgis-integration': ['postgis'],
            'use-proj': ['proj'],
            'use-serde': ['serde', 'geo-types/serde'],
          },
          yanked: false,
          license: 'MIT/Apache-2.0',
          links: {
            dependencies: '/api/v1/crates/geo/0.12.2/dependencies',
            version_downloads: '/api/v1/crates/geo/0.12.2/downloads',
            authors: '/api/v1/crates/geo/0.12.2/authors',
          },
          crate_size: 179841,
          published_by: {
            id: 227,
            login: 'frewsxcv',
            name: 'Corey Farwell',
            avatar: 'https://avatars2.githubusercontent.com/u/416575?v=4',
            url: 'https://github.com/frewsxcv',
          },
          audit_actions: [],
        },
        {
          id: 143262,
          crate: 'geo',
          num: '0.12.1',
          dl_path: '/api/v1/crates/geo/0.12.1/download',
          readme_path: '/api/v1/crates/geo/0.12.1/readme',
          updated_at: '2019-04-05T09:00:59.629392+00:00',
          created_at: '2019-04-05T09:00:59.629392+00:00',
          downloads: 2940,
          features: {
            default: [],
            'postgis-integration': ['postgis'],
            'use-proj': ['proj'],
            'use-serde': ['serde', 'geo-types/serde'],
          },
          yanked: false,
          license: 'MIT/Apache-2.0',
          links: {
            dependencies: '/api/v1/crates/geo/0.12.1/dependencies',
            version_downloads: '/api/v1/crates/geo/0.12.1/downloads',
            authors: '/api/v1/crates/geo/0.12.1/authors',
          },
          crate_size: 179259,
          published_by: {
            id: 227,
            login: 'frewsxcv',
            name: 'Corey Farwell',
            avatar: 'https://avatars2.githubusercontent.com/u/416575?v=4',
            url: 'https://github.com/frewsxcv',
          },
          audit_actions: [],
        },
        {
          id: 134231,
          crate: 'geo',
          num: '0.12.0',
          dl_path: '/api/v1/crates/geo/0.12.0/download',
          readme_path: '/api/v1/crates/geo/0.12.0/readme',
          updated_at: '2019-02-17T03:19:08.118477+00:00',
          created_at: '2019-02-17T03:19:08.118477+00:00',
          downloads: 4420,
          features: {
            default: [],
            'postgis-integration': ['postgis'],
            'use-proj': ['proj'],
            'use-serde': ['serde', 'geo-types/serde'],
          },
          yanked: false,
          license: 'MIT/Apache-2.0',
          links: {
            dependencies: '/api/v1/crates/geo/0.12.0/dependencies',
            version_downloads: '/api/v1/crates/geo/0.12.0/downloads',
            authors: '/api/v1/crates/geo/0.12.0/authors',
          },
          crate_size: 178368,
          published_by: null,
          audit_actions: [],
        },
      ],
      meta: { more: true },
    });

    this.server.get(`/api/v1/users/${user.id}/stats`, { total_downloads: 3892 });

    await visit('/dashboard');
    assert.equal(currentURL(), '/dashboard');
    percySnapshot(assert);
  });
});
