import { test } from 'qunit';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';

moduleForAcceptance('Acceptance | crate page');

test('visiting a crate page from the front page', async function(assert) {
    server.create('crate', 'withVersion', { id: 'nanomsg' });

    await visit('/');
    await click('[data-test-just-updated] [data-test-crate-link="0"]');

    assert.equal(currentURL(), '/crates/nanomsg');
    assert.equal(document.title, 'nanomsg - Cargo: packages for Rust');
});

test('visiting /crates/nanomsg', async function(assert) {
    server.create('crate', { id: 'nanomsg', max_version: '0.6.1' });
    server.create('version', { crate: 'nanomsg', num: '0.6.0' });
    server.create('version', { crate: 'nanomsg', num: '0.6.1' });

    await visit('/crates/nanomsg');

    assert.equal(currentURL(), '/crates/nanomsg');
    assert.equal(currentRouteName(), 'crate.index');
    assert.equal(document.title, 'nanomsg - Cargo: packages for Rust');

    assert.dom('[data-test-heading] [data-test-crate-name]').hasText('nanomsg');
    assert.dom('[data-test-heading] [data-test-crate-version]').hasText('0.6.1');
});

test('visiting /crates/nanomsg/', async function(assert) {
    server.create('crate', { id: 'nanomsg', max_version: '0.6.1' });
    server.create('version', { crate: 'nanomsg', num: '0.6.0' });
    server.create('version', { crate: 'nanomsg', num: '0.6.1' });

    await visit('/crates/nanomsg/');

    assert.equal(currentURL(), '/crates/nanomsg/');
    assert.equal(currentRouteName(), 'crate.index');
    assert.equal(document.title, 'nanomsg - Cargo: packages for Rust');

    assert.dom('[data-test-heading] [data-test-crate-name]').hasText('nanomsg');
    assert.dom('[data-test-heading] [data-test-crate-version]').hasText('0.6.1');
});

test('visiting /crates/nanomsg/0.6.0', async function(assert) {
    server.create('crate', { id: 'nanomsg', max_version: '0.6.1' });
    server.create('version', { crate: 'nanomsg', num: '0.6.0' });
    server.create('version', { crate: 'nanomsg', num: '0.6.1' });

    await visit('/crates/nanomsg/0.6.0');

    assert.equal(currentURL(), '/crates/nanomsg/0.6.0');
    assert.equal(currentRouteName(), 'crate.version');
    assert.equal(document.title, 'nanomsg - Cargo: packages for Rust');

    assert.dom('[data-test-heading] [data-test-crate-name]').hasText('nanomsg');
    assert.dom('[data-test-heading] [data-test-crate-version]').hasText('0.6.0');
});

test('navigating to the all versions page', async function(assert) {
    server.loadFixtures();

    await visit('/crates/nanomsg');
    await click('[data-test-all-versions-link]');

    assert.dom('.info').hasText(/All 13\s+versions of nanomsg since\s+December \d+, 2014/);
});

test('navigating to the reverse dependencies page', async function(assert) {
    server.loadFixtures();

    await visit('/crates/nanomsg');
    await click('[data-test-reverse-deps-link]');

    assert.equal(currentURL(), '/crates/nanomsg/reverse_dependencies');
    assert.dom('a[href="/crates/unicorn-rpc"]').hasText('unicorn-rpc');
});

test('navigating to a user page', async function(assert) {
    server.loadFixtures();

    await visit('/crates/nanomsg');
    await click('[data-test-owners] [data-test-user-link="blabaere"]');

    assert.equal(currentURL(), '/users/blabaere');
    assert.dom('[data-test-heading] [data-test-username]').hasText('blabaere');
});

test('navigating to a team page', async function(assert) {
    server.loadFixtures();

    await visit('/crates/nanomsg');
    await click('[data-test-owners] [data-test-team-link="github:org:thehydroimpulse"]');

    assert.equal(currentURL(), '/teams/github:org:thehydroimpulse');
    assert.dom('[data-test-heading] [data-test-team-name]').hasText('thehydroimpulseteam');
});

test('crates having user-owners', async function(assert) {
    server.loadFixtures();

    await visit('/crates/nanomsg');

    assert.dom('[data-test-owners] [data-test-team-link="github:org:thehydroimpulse"] img')
        .hasAttribute('src', 'https://avatars.githubusercontent.com/u/565790?v=3&s=64');

    assert.dom('[data-test-owners] li').exists({ count: 4 });
});

test('crates having team-owners', async function(assert) {
    server.loadFixtures();

    await visit('/crates/nanomsg');

    assert.dom('[data-test-owners] [data-test-team-link="github:org:thehydroimpulse"]').exists();
    assert.dom('[data-test-owners] li').exists({ count: 4 });
});

test('crates license is supplied by version', async function(assert) {
    server.loadFixtures();

    await visit('/crates/nanomsg');
    assert.dom('[data-test-license]').hasText('Apache-2.0');

    await click('[data-test-version-link="0.5.0"]');
    assert.dom('[data-test-license]').hasText('MIT/Apache-2.0');
});
