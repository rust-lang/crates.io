import { Factory } from 'miragejs';

const ORGS = ['rust-lang', 'emberjs', 'rust-random', 'georust', 'actix'];

export default Factory.extend({
  name: i => `team-${i + 1}`,
  org: i => ORGS[i % ORGS.length],

  login() {
    return `github:${this.org}:${this.name}`;
  },

  url() {
    return `https://github.com/${this.org}`;
  },

  avatar: 'https://avatars1.githubusercontent.com/u/14631425?v=4',
});
