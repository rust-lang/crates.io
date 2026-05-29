<script lang="ts">
  import { resolve } from '$app/paths';

  import Icon from '$lib/components/Icon.svelte';
</script>

<footer class="footer">
  <div class="content width-limit">
    <div>
      <h2>Rust</h2>
      <ul role="list">
        <li><a href="https://www.rust-lang.org/">rust-lang.org</a></li>
        <li><a href="https://foundation.rust-lang.org/">Rust Foundation</a></li>
        <li><a href="https://www.rust-lang.org/governance/teams/dev-tools#team-crates-io">The crates.io team</a></li>
      </ul>
    </div>

    <div>
      <h2>Get Help</h2>
      <ul role="list">
        <li><a href="https://doc.rust-lang.org/cargo/">The Cargo Book</a></li>
        <li><a href={resolve('/support')} data-test-support-link>Support</a></li>
        <li><a href="https://status.crates.io/">System Status</a></li>
        <li><a href="https://github.com/rust-lang/crates.io/issues/new/choose">Report a bug</a></li>
      </ul>
    </div>

    <div>
      <h2>Policies</h2>
      <ul role="list">
        <li><a href={resolve('/policies')}>Usage Policy</a></li>
        <li><a href={resolve('/policies/security')}>Security</a></li>
        <li><a href="https://foundation.rust-lang.org/policies/privacy-policy/">Privacy Policy</a></li>
        <li><a href="https://www.rust-lang.org/policies/code-of-conduct">Code of Conduct</a></li>
        <li><a href={resolve('/data-access')}>Data Access</a></li>
        <li><a href={resolve('/docs/rate-limits')}>Rate Limits</a></li>
      </ul>
    </div>

    <div>
      <h2>Social</h2>
      <ul role="list">
        <li>
          <a href="https://github.com/rust-lang/crates.io/">
            <Icon class="i-simple-icons:github" />
            <span class="sr-only">GitHub: </span>rust-lang/crates.io
          </a>
        </li>
        <li>
          <a href="https://rust-lang.zulipchat.com/#streams/318791/t-crates-io">
            <Icon class="i-simple-icons:zulip" />
            <span class="sr-only">Zulip: </span>#t-crates-io
          </a>
        </li>
        <li>
          <a href="https://twitter.com/cratesiostatus">
            <Icon class="i-simple-icons:x" />
            <span class="sr-only">X: </span>@cratesiostatus
          </a>
        </li>
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

    & h2 {
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

        & :global(.icon) {
          filter: drop-shadow(1px 1px 1px var(--footer-link-hover-shadow-color));
        }
      }
    }

    & :global(.icon) {
      margin: 0 var(--space-2xs) 0 0;
    }
  }
</style>
