<script lang="ts">
  import CrateHeader from '$lib/components/CrateHeader.svelte';

  let { data } = $props();

  function aliasUrl(alias: string): string | null {
    if (alias.startsWith('CVE-')) {
      return `https://nvd.nist.gov/vuln/detail/${alias}`;
    } else if (alias.startsWith('GHSA-')) {
      return `https://github.com/advisories/${alias}`;
    }
    return null;
  }

  function cvssUrl(cvss: string): string | null {
    let match = cvss.match(/^CVSS:(\d+\.\d+)\//);
    if (match) {
      return `https://www.first.org/cvss/calculator/${match[1]}#${cvss}`;
    }
    return null;
  }
</script>

<CrateHeader crate={data.crate} keywords={data.keywords} ownersPromise={data.ownersPromise} />

{#if data.advisories.length}
  <h2 class="heading">Advisories</h2>
  <ul class="advisories" data-test-list>
    {#each data.advisories as advisory (advisory.id)}
      <li class="row">
        <h3>
          <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
          <a href="https://rustsec.org/advisories/{advisory.id}.html">{advisory.id}</a>:
          {advisory.summary}
        </h3>
        {#if advisory.versionRanges}
          <div class="affected-versions" data-test-affected-versions>
            <strong>Affected versions:</strong>
            {advisory.versionRanges}
          </div>
        {/if}
        {#if advisory.aliases?.length}
          <div class="aliases" data-test-aliases>
            <strong>Aliases:</strong>
            <ul>
              {#each advisory.aliases as alias (alias)}
                <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
                <li><a href={aliasUrl(alias)}>{alias}</a></li>
              {/each}
            </ul>
          </div>
        {/if}
        {#if advisory.cvss}
          <div class="cvss" data-test-cvss>
            <strong>CVSS:</strong>
            <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -->
            <a href={cvssUrl(advisory.cvss)}>{advisory.cvss}</a>
          </div>
        {/if}
        <!-- eslint-disable-next-line svelte/no-at-html-tags -->
        {@html data.convertMarkdown(advisory.details)}
      </li>
    {/each}
  </ul>
{:else}
  <div class="no-results" data-no-advisories>No advisories found for this crate.</div>
{/if}

<style>
  .heading {
    font-size: 1.17em;
    margin-block-start: 1em;
    margin-block-end: 1em;
  }

  .advisories {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .row {
    margin-top: var(--space-2xs);
    background-color: light-dark(white, #141413);
    border-radius: var(--space-3xs);
    padding: var(--space-m) var(--space-l);
    list-style: none;
    overflow-wrap: break-word;
  }

  .no-results {
    padding: var(--space-l) var(--space-s);
    background-color: light-dark(white, #141413);
    text-align: center;
    font-size: 20px;
    font-weight: 300;
    overflow-wrap: break-word;
    line-height: 1.5;
  }

  .affected-versions {
    margin-top: var(--space-s);
    margin-bottom: var(--space-m);
    padding: var(--space-xs) var(--space-s);
    background-color: light-dark(#f5f5f5, #1a1a19);
    border-left: 3px solid var(--orange-500);
    border-radius: var(--space-4xs);
  }

  .affected-versions strong {
    margin-right: var(--space-2xs);
  }

  .aliases {
    margin-top: var(--space-s);
    margin-bottom: var(--space-m);
  }

  .aliases ul {
    margin: var(--space-2xs) 0 0 var(--space-m);
    padding: 0;
    font-family: monospace;
    font-size: 0.9em;
  }

  .aliases li {
    margin: var(--space-3xs) 0;
  }

  .cvss {
    margin-top: var(--space-s);
    margin-bottom: var(--space-m);
  }

  .cvss strong {
    margin-right: var(--space-2xs);
  }
</style>
