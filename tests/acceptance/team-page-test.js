import { test } from 'qunit';
import moduleForAcceptance from 'cargo/tests/helpers/module-for-acceptance';
import hasText from 'cargo/tests/helpers/has-text';

moduleForAcceptance('Acceptance | team page');

test('has team organization display', async function(assert) {
    await visit('/teams/github:org:thehydroimpulse');

    hasText(assert, '.team-info h1', 'org');
    hasText(assert, '.team-info h2', 'thehydroimpulseteam');
});

test('has link to github in team header', async function(assert) {
    await visit('/teams/github:org:thehydroimpulse');

    const $githubLink = findWithAssert('.info a');
    assert.equal($githubLink.attr('href').trim(), 'https://github.com/org_test');
});

test('github link has image in team header', async function(assert) {
    await visit('/teams/github:org:thehydroimpulse');

    const $githubImg = findWithAssert('.info a img');
    assert.equal($githubImg.attr('src').trim(), '/assets/GitHub-Mark-32px.png');
});

test('team organization details has github profile icon', async function(assert) {
    await visit('/teams/github:org:thehydroimpulse');

    const $githubProfileImg = findWithAssert('.info img');
    assert.equal($githubProfileImg.attr('src').trim(), 'https://avatars.githubusercontent.com/u/565790?v=3&s=170');
});
