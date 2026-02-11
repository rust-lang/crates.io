<script lang="ts">
  import { highlightSyntax } from '$lib/attachments/highlight';
  import PageHeader from '$lib/components/PageHeader.svelte';
  import TextContent from '$lib/components/TextContent.svelte';

  // Defined as a variable to avoid Svelte parsing issues with
  // curly braces and backslash escapes in inline code blocks.
  let exchangeTokenScript = `#!/bin/bash
set -e

# Exchange JWT token
echo "Exchanging OIDC token..." >&2
RESPONSE=$(curl -s -X POST https://crates.io/api/v1/trusted_publishing/tokens \\
  -H "Content-Type: application/json" \\
  -d "{\\"jwt\\": \\"$CRATES_IO_ID_TOKEN\\"}")

# Extract publish token
CRATES_IO_PUBLISH_TOKEN=$(echo "$RESPONSE" | jq -r '.token')

if [ "$CRATES_IO_PUBLISH_TOKEN" = "null" ] || [ -z "$CRATES_IO_PUBLISH_TOKEN" ]; then
  echo "Failed to get upload token" >&2
  echo "$RESPONSE" >&2
  exit 1
fi

echo "$CRATES_IO_PUBLISH_TOKEN"`;
</script>

<PageHeader title="Trusted Publishing" />

<TextContent boxed>
  <div>
    <h2>What is Trusted Publishing?</h2>
    <p>
      Trusted Publishing is a secure way to publish your Rust crates from CI/CD platforms like GitHub Actions and GitLab
      CI/CD without manually managing API tokens. It uses OpenID Connect (OIDC) to verify that your workflow is running
      from your repository, then provides a short-lived token for publishing.
    </p>

    <p>
      Instead of storing long-lived API tokens in your repository secrets, Trusted Publishing allows your CI/CD platform
      to authenticate directly with crates.io using cryptographically signed tokens that prove the workflow's identity.
    </p>

    <p>
      <strong>Note:</strong>
      GitLab CI/CD support is currently in public beta.
    </p>

    <h3>Security Benefits</h3>
    <ul>
      <li><strong>No long-lived API tokens</strong> to manage or rotate</li>
      <li><strong>Tokens automatically expire</strong> after 30 minutes</li>
      <li><strong>Repository and workflow verification</strong> prevents unauthorized publishing</li>
      <li><strong>OIDC-based cryptographic verification</strong> with your platform's public JWKS</li>
      <li><strong>Optional environments</strong> for additional access controls</li>
    </ul>

    <h2>Quick Start</h2>
    <p>Follow these steps to set up Trusted Publishing for your crate:</p>

    <ol>
      <li><strong>Configure your crate for Trusted Publishing</strong> in the crates.io settings</li>
      <li>
        <strong>Set up your CI/CD workflow</strong>
        with the required permissions and authentication
      </li>
      <li><strong>Publish your crate</strong> using the automated workflow</li>
    </ol>

    <h3>Prerequisites</h3>
    <ul>
      <li>Your crate must already be published to crates.io (initial publish requires an API token)</li>
      <li>You must be an owner of the crate on crates.io</li>
      <li>Your repository must be on GitHub or GitLab</li>
    </ul>

    <h2>Configuring Trusted Publishing</h2>
    <p>Configure your crate on crates.io:</p>

    <ol>
      <li>Go to your crate's Settings → Trusted Publishing</li>
      <li>Click the "Add" button and select your platform (GitHub or GitLab)</li>
      <li>Fill in the platform-specific fields and save the configuration</li>
    </ol>

    <h3>GitHub Configuration</h3>
    <ul>
      <li><strong>Repository owner:</strong> Your GitHub username or organization</li>
      <li><strong>Repository name:</strong> The name of your repository</li>
      <li>
        <strong>Workflow filename:</strong>
        The filename of your GitHub Actions workflow (e.g., "release.yml")
      </li>
      <li><strong>Environment:</strong> Optional environment name if you're using GitHub environments</li>
    </ul>

    <h3>GitLab Configuration</h3>
    <ul>
      <li><strong>Namespace:</strong> Your GitLab username or group path</li>
      <li><strong>Project:</strong> The name of your project</li>
      <li>
        <strong>Workflow filepath:</strong>
        The full filepath of your GitLab CI/CD workflow (e.g., "ci/release.yml")
      </li>
      <li><strong>Environment:</strong> Optional environment name if you're using GitLab CI/CD environments</li>
    </ul>

    <h2>GitHub Actions Setup</h2>
    <p>
      Create a workflow file at
      <code>.github/workflows/release.yml</code>. This example workflow will automatically publish your crate each time
      you push a version tag (like
      <code>v1.0.0</code>):
    </p>

    <!-- prettier-ignore -->
    <pre><code class="language-yaml" {@attach highlightSyntax()}>name: Publish to crates.io
on:
  push:
    tags: ['v*']  # Triggers when pushing tags starting with 'v'
jobs:
  publish:
    runs-on: ubuntu-latest
    environment: release  # Optional: for enhanced security
    permissions:
      id-token: write     # Required for OIDC token exchange
    steps:
    - uses: actions/checkout@v6
    - uses: rust-lang/crates-io-auth-action@v1
      id: auth
    - run: cargo publish
      env:
        CARGO_REGISTRY_TOKEN: {'${{ steps.auth.outputs.token }}'}</code></pre>

    <p>
      <strong>Optional:</strong>
      For enhanced security, create a GitHub Actions environment named "release" in your repository settings with protection
      rules like required reviewers or deployment branches.
    </p>

    <h2>GitLab CI/CD Setup <small>(Public Beta)</small></h2>
    <p>
      Create a workflow file at
      <code>.gitlab-ci.yml</code>. This example workflow will automatically publish your crate each time you push a
      version tag (like
      <code>v1.0.0</code>):
    </p>

    <!-- prettier-ignore -->
    <pre><code class="language-yaml" {@attach highlightSyntax()}>publish:
  image: rust:1.91.0-alpine
  environment: release
  only:
    - tags  # Only run on tag pushes
  id_tokens:
    CRATES_IO_ID_TOKEN:
      aud: crates.io
  before_script:
    - apk add --no-cache bash curl jq
  script:
    # Exchange OIDC token for publish token
    - CARGO_REGISTRY_TOKEN=$(bash exchange-token.sh)
    # Publish to crates.io
    - CARGO_REGISTRY_TOKEN="$CARGO_REGISTRY_TOKEN" cargo publish</code></pre>

    <p>
      Create a helper script at
      <code>exchange-token.sh</code>
      in your repository root:
    </p>

    <pre><code class="language-bash" {@attach highlightSyntax()}>{exchangeTokenScript}</code></pre>

    <p>
      <strong>Optional:</strong>
      For enhanced security, create a GitLab CI/CD environment named "release" in your repository settings with protection
      rules like required reviewers or deployment branches.
    </p>

    <h2>Security &amp; Best Practices</h2>
    <ul>
      <li><strong>Use specific workflow filenames</strong> to reduce the attack surface</li>
      <li><strong>Use environments with protection rules</strong> for sensitive publishing</li>
      <li><strong>Limit workflow triggers</strong> to specific tags or protected branches</li>
      <li><strong>Review all actions used</strong> in your release workflow</li>
      <li><strong>Monitor publishing activities</strong> through crates.io email notifications</li>
    </ul>

    <p>
      <strong>How it works:</strong>
      Your CI/CD platform generates an OIDC token that proves your workflow's identity. For GitHub, the
      <code>rust-lang/crates-io-auth-action</code>
      exchanges this for a 30-minute access token. For GitLab, the provided script exchanges the token via the crates.io API.
      <code>cargo publish</code>
      uses this token automatically.
    </p>

    <h2>Migration from API Tokens</h2>
    <p>
      To migrate from API tokens: set up Trusted Publishing following the steps above, test it, then remove the API
      token from your repository secrets. Both methods can be used simultaneously during transition.
    </p>

    <h2>Additional Resources</h2>
    <ul>
      <li>
        <a href="https://rust-lang.github.io/rfcs/3691-trusted-publishing-cratesio.html">
          RFC 3691: Trusted Publishing for crates.io
        </a>
      </li>
      <li>
        <a
          href="https://docs.github.com/en/actions/deployment/security-hardening-your-deployments/about-security-hardening-with-openid-connect"
        >
          GitHub: About security hardening with OpenID Connect
        </a>
      </li>
      <li>
        <a
          href="https://docs.github.com/en/actions/deployment/targeting-different-environments/using-environments-for-deployment"
        >
          GitHub: Using environments for deployment
        </a>
      </li>
      <li>
        <a href="https://docs.gitlab.com/ee/ci/secrets/id_token_authentication.html">
          GitLab: OpenID Connect (OIDC) authentication
        </a>
      </li>
      <li>
        <a href="https://docs.gitlab.com/ee/ci/environments/"> GitLab: Environments and deployments </a>
      </li>
    </ul>
  </div>
</TextContent>
