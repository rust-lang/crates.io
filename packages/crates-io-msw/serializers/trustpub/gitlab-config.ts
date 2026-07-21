import type { components } from '@crates-io/api-client';
import type { TrustpubGitlabConfig } from '../../models/index.js';

type ApiGitLabConfig = components['schemas']['GitLabConfig'];

export function serializeGitLabConfig(config: TrustpubGitlabConfig): ApiGitLabConfig {
  return {
    id: config.id,
    crate: config.crate.name,
    namespace: config.namespace,
    namespace_id: config.namespace_id,
    project: config.project,
    workflow_filepath: config.workflow_filepath,
    environment: config.environment,
    created_at: config.created_at,
  };
}
