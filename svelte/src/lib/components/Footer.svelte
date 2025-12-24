<script lang="ts">
  import { resolve } from '$app/paths';

  import GitHub from '$lib/assets/github.svg?component';
  import Twitter from '$lib/assets/twitter.svg?component';
  import Zulip from '$lib/assets/zulip.svg?component';
</script>

<footer class="footer">
  <div class="content width-limit">
    <div>
      <h1>Rust</h1>
      <ul role="list">
        <li><a href="https://www.rust-lang.org/">rust-lang.org</a></li>
        <li><a href="https://foundation.rust-lang.org/">Rust Foundation</a></li>
        <li><a href="https://www.rust-lang.org/governance/teams/dev-tools#team-crates-io">The crates.io team</a></li>
      </ul>
    </div>

    <div>
      <h1>Get Help</h1>
      <ul role="list">
        <li><a href="https://doc.rust-lang.org/cargo/">The Cargo Book</a></li>
        <li><a href={resolve('/support')}>Support</a></li>
        <li><a href="https://status.crates.io/">System Status</a></li>
        <li><a href="https://github.com/rust-lang/crates.io/issues/new/choose">Report a bug</a></li>
      </ul>
    </div>

    <div>
      <h1>Policies</h1>
      <ul role="list">
        <li><a href={resolve('/policies')}>Usage Policy</a></li>
        <li><a href={resolve('/policies/security')}>Security</a></li>
        <li><a href="https://foundation.rust-lang.org/policies/privacy-policy/">Privacy Policy</a></li>
        <li><a href="https://www.rust-lang.org/policies/code-of-conduct">Code of Conduct</a></li>
        <li><a href={resolve('/data-access')}>Data Access</a></li>
      </ul>
    </div>

    <div>
      <h1>Social</h1>
      <ul role="list">
        <li><a href="https://github.com/rust-lang/crates.io/"><GitHub /> rust-lang/crates.io</a></li>
        <li><a href="https://rust-lang.zulipchat.com/#streams/318791/t-crates-io"><Zulip /> #t-crates-io</a></li>
        <li><a href="https://twitter.com/cratesiostatus"><Twitter /> @cratesiostatus</a></li>
      </ul>
    </div>
  </div>
</footer>

<style>
  :root,
  :global([data-theme='classic']) {
    --footer-bg-color: var(--header-bg-color);
    --footer-header-color: var(--yellow500);
    --footer-header-shadow-color: var(--green900);
    --footer-link-color: #fff;
    --footer-link-hover-color: var(--yellow500);
    --footer-link-hover-shadow-color: var(--green900);
  }

  .footer {
    display: grid;
    justify-items: center;
    background: var(--footer-bg-color);
  }

  .content {
    display: grid;
    gap: var(--space-xl) var(--space-m);
    padding: var(--space-l);

    @media only screen and (min-width: 551px) {
      grid-template-columns: repeat(2, 1fr);
      justify-content: center;
      padding: var(--space-xl) var(--space-m);
    }

    @media only screen and (min-width: 751px) {
      grid-template-columns: repeat(4, 1fr);
    }

    & h1 {
      margin: 0 0 var(--space-s);
      font-size: 20px;
      font-weight: 500;
      color: var(--footer-header-color);
      text-shadow: 1px 1px 1px var(--footer-header-shadow-color);
    }

    & ul {
      list-style: none;
      margin: 0;
      padding: 0;

      > * + * {
        margin-top: var(--space-xs);
      }
    }

    & li {
      height: 1rem;
    }

    & a {
      display: inline-flex;
      /* slightly increased click targets */
      margin: calc(var(--space-2xs) * -1);
      padding: var(--space-2xs);
      color: #fff;
      font-size: 16px;
      line-height: 1em;
      white-space: nowrap;
      transition: var(--transition-medium);

      &:hover {
        color: var(--footer-link-hover-color);
        text-shadow: 1px 1px 1px var(--footer-link-hover-shadow-color);
        /* apply color fade only on mouse-out */
        transition: var(--transition-instant);

        & :global(svg) {
          filter: drop-shadow(1px 1px 1px var(--footer-link-hover-shadow-color));
        }
      }
    }

    & :global(svg) {
      height: 1.25em;
      width: 1.25em;
      margin: -0.125em var(--space-xs) -0.125em 0;
      flex-shrink: 0;
    }
  }
</style>
