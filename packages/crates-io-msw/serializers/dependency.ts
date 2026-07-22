import type { components } from '@crates-io/api-client';
import type { Dependency } from '../models/index.js';

type ApiDependency = components['schemas']['Dependency'];

export function serializeDependency(dependency: Dependency): ApiDependency {
  return {
    id: dependency.id,
    crate_id: dependency.crate.name,
    version_id: dependency.version.id,
    req: dependency.req,
    optional: dependency.optional,
    default_features: dependency.default_features,
    downloads: dependency.crate.downloads,
    features: dependency.features,
    kind: dependency.kind,
    target: dependency.target,
  };
}
