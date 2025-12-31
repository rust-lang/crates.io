<script lang="ts">
  import type { HTMLAttributes } from 'svelte/elements';

  import { resolve } from '$app/paths';

  const REASONS = [
    {
      reason: 'spam',
      description: 'it contains spam',
    },
    {
      reason: 'name-squatting',
      description: 'it is name-squatting (reserving a crate name without content)',
    },
    {
      reason: 'abuse',
      description: 'it is abusive or otherwise harmful',
    },
    {
      reason: 'malicious-code',
      description: 'it contains malicious code',
    },
    {
      reason: 'vulnerability',
      description: 'it contains a vulnerability',
    },
    {
      reason: 'other',
      description: 'it is violating the usage policy in some other way (please specify below)',
    },
  ];

  interface Props extends HTMLAttributes<HTMLFormElement> {
    crate?: string;
  }

  let { crate: initialCrate = '', ...restProps }: Props = $props();
  let id = $props.id();

  // svelte-ignore state_referenced_locally
  let crate = $state(initialCrate);
  let selectedReasons = $state<string[]>([]);
  let detail = $state('');

  let crateInvalid = $state(false);
  let reasonsInvalid = $state(false);
  let detailInvalid = $state(false);

  let isMaliciousCodeReport = $derived(selectedReasons.includes('malicious-code'));
  let isVulnerabilityReport = $derived(selectedReasons.includes('vulnerability'));

  function validate(): boolean {
    crateInvalid = !crate || !crate.trim();
    reasonsInvalid = selectedReasons.length === 0;
    detailInvalid = selectedReasons.includes('other') && !detail?.trim();
    return !crateInvalid && !reasonsInvalid && !detailInvalid;
  }

  function isReasonSelected(reason: string): boolean {
    return selectedReasons.includes(reason);
  }

  function toggleReason(reason: string): void {
    selectedReasons = selectedReasons.includes(reason)
      ? selectedReasons.filter(it => it !== reason)
      : [...selectedReasons, reason];
    reasonsInvalid = false;
  }

  function composeMail(): string {
    let reasons = REASONS.map(({ reason, description }) => {
      let selected = isReasonSelected(reason);
      return `${selected ? '- [x]' : '- [ ]'} ${description}`;
    }).join('\n');

    let body = `I'm reporting the https://crates.io/crates/${crate} crate because:

${reasons}

Additional details:

${detail}
`;
    let subject = `The "${crate}" crate`;
    if (isMaliciousCodeReport) {
      subject = `[SECURITY] ${subject}`;
    }

    let addresses = 'help@crates.io';
    if (isMaliciousCodeReport) {
      addresses += ',security@rust-lang.org';
    }

    return `mailto:${addresses}?subject=${encodeURIComponent(subject)}&body=${encodeURIComponent(body)}`;
  }

  function handleSubmit(event: SubmitEvent): void {
    event.preventDefault();

    if (!validate()) {
      return;
    }

    let mailto = composeMail();
    window.open(mailto, '_self');
  }
</script>

<form {...restProps} class="report-form" data-testid="crate-report-form" onsubmit={handleSubmit}>
  <h2>Report A Crate</h2>

  <fieldset class="form-group" data-test-id="fieldset-crate">
    <label for="{id}-crate" class="form-group-name">Crate</label>
    <!-- svelte-ignore a11y_autofocus -->
    <input
      id="{id}-crate"
      type="text"
      bind:value={crate}
      autocomplete="off"
      aria-required="true"
      aria-invalid={crateInvalid}
      class="crate-input base-input"
      data-test-id="crate-input"
      autofocus
      oninput={() => (crateInvalid = false)}
    />
    {#if crateInvalid}
      <div class="form-group-error" data-test-id="crate-invalid">Please specify a crate.</div>
    {/if}
  </fieldset>

  <fieldset class="form-group" data-test-id="fieldset-reasons">
    <div class="form-group-name">Reasons</div>
    <ul role="list" class="reasons-list scopes-list" class:invalid={reasonsInvalid}>
      {#each REASONS as option (option.reason)}
        <li>
          <label>
            <input
              type="checkbox"
              checked={isReasonSelected(option.reason)}
              name={option.reason}
              data-test-id="{option.reason}-checkbox"
              onchange={() => toggleReason(option.reason)}
            />
            {option.description}
          </label>
        </li>
      {/each}
    </ul>
    {#if reasonsInvalid}
      <div class="form-group-error" data-test-id="reasons-invalid">Please choose reasons to report.</div>
    {/if}
  </fieldset>

  {#if isVulnerabilityReport}
    <div class="vulnerability-report form-group" data-test-id="vulnerability-report">
      <h3>🔍 Vulnerability Report</h3>
      <p>For crate vulnerabilities, please consider:</p>
      <ul>
        <li>Contacting the crate author first when possible</li>
        <li>
          Reporting to the
          <a href="https://rustsec.org/contributing.html" target="_blank" rel="noopener noreferrer">
            RustSec Advisory Database
          </a>
        </li>
        <li>
          Reviewing our <a href={resolve('/policies/security')} target="_blank">security policy</a>
        </li>
      </ul>
    </div>
  {/if}

  <fieldset class="form-group" data-test-id="fieldset-detail">
    <label for="{id}-detail" class="form-group-name">Detail</label>
    <textarea
      id="{id}-detail"
      bind:value={detail}
      class="detail"
      class:invalid={detailInvalid}
      aria-required={selectedReasons.includes('other')}
      aria-invalid={detailInvalid}
      rows="5"
      data-test-id="detail-input"
      oninput={() => (detailInvalid = false)}
    ></textarea>
    {#if detailInvalid}
      <div class="form-group-error" data-test-id="detail-invalid">Please provide some detail.</div>
    {/if}
  </fieldset>

  <div class="buttons">
    <button type="submit" class="report-button button button--small" data-test-id="report-button">
      Report to
      {#if isMaliciousCodeReport}
        <strong>help@crates.io & security@rust-lang.org</strong>
      {:else}
        <strong>help@crates.io</strong>
      {/if}
    </button>
  </div>
</form>

<style>
  .report-form {
    background-color: var(--main-bg);
    padding: 0.5rem 1rem;
  }

  .form-group {
    border: none;
    margin: 0;
    padding: 0;

    & + & {
      margin-top: 1rem;
    }
  }

  .crate-input {
    max-width: 440px;
    width: 100%;
  }

  .reasons-list {
    list-style: none;
    padding: 0;
    margin: 0;
    background-color: light-dark(white, #141413);
    border: 1px solid var(--gray-border);
    border-radius: var(--space-3xs);

    input {
      align-self: center;
    }

    &.invalid {
      background: light-dark(#fff2f2, #170808);
      border-color: red;
    }

    > * + * {
      border-top: inherit;
    }

    label {
      padding: var(--space-xs) var(--space-s);
      display: flex;
      flex-wrap: nowrap;
      gap: var(--space-xs);
      font-size: 0.9em;
    }
  }

  .detail {
    padding: var(--space-2xs);
    background-color: light-dark(white, #141413);
    border: 1px solid var(--gray-border);
    border-radius: var(--space-3xs);
    resize: vertical;
    width: 100%;

    &.invalid {
      background: light-dark(#fff2f2, #170808);
      border-color: red;
    }
  }

  .vulnerability-report {
    padding: var(--space-s) var(--space-s);
    background-color: light-dark(white, #141413);
    border: 1px solid var(--gray-border);
    border-radius: var(--space-3xs);
    width: 100%;

    :first-child {
      margin-top: 0;
    }

    :last-child {
      margin-bottom: 0;
    }
  }

  .buttons {
    position: relative;
    margin: var(--space-m) 0;
    display: flex;
    flex-wrap: wrap;
    justify-content: end;
    gap: 2rem;
  }

  .report-button {
    border-radius: var(--space-3xs);
    font-weight: normal;

    &:focus {
      outline: 1px solid var(--bg-color-top-dark);
      outline-offset: 2px;
    }

    strong {
      margin-left: var(--space-3xs);
      font-weight: 500;
    }
  }
</style>
