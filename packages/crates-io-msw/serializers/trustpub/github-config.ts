import type { components } from '@crates-io/api-client';
import type { TrustpubGithubConfig } from '../../models/index.js';

type ApiGitHubConfig = components['schemas']['GitHubConfig'];

export function serializeGitHubConfig(config: TrustpubGithubConfig): ApiGitHubConfig {
  return {
    id: config.id,
    crate: config.crate.name,
    repository_owner: config.repository_owner,
    repository_owner_id: config.repository_owner_id,
    repository_name: config.repository_name,
    workflow_filename: config.workflow_filename,
    environment: config.environment,
    created_at: config.created_at,
  };
}
