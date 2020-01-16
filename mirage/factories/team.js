import { Factory } from 'ember-cli-mirage';

const ORGS = ['rust-lang', 'emberjs', 'rust-random', 'georust', 'actix'];

export default Factory.extend({
  name: i => `team-${i + 1}`,

  login(i) {
    return `github:${ORGS[i % ORGS.length]}:${this.name}`;
  },

  url(i) {
    return `https://github.com/${ORGS[i % ORGS.length]}`;
  },

  avatar: 'https://avatars1.githubusercontent.com/u/14631425?v=4',
});
