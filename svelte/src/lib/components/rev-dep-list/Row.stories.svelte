<script module lang="ts">
  import type { components } from '@crates-io/api-client';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import Row from './Row.svelte';

  type Dependency = components['schemas']['Dependency'] & {
    dependentCrateName: string;
  };

  const { Story } = defineMeta({
    title: 'rev-dep-list/Row',
    component: Row,
  });

  function createDependency(overrides: Partial<Dependency> = {}): Dependency {
    return {
      id: 1,
      crate_id: 'tokio',
      version_id: 42,
      req: '^1.0',
      kind: 'normal',
      optional: false,
      default_features: true,
      features: [],
      downloads: 100000,
      dependentCrateName: 'actix-web',
      ...overrides,
    };
  }

  let resolved = (value: string | null) => Promise.resolve(value);
  let pending = () => new Promise<string | null>(() => {});
  let rejected = () => Promise.reject(new Error('failed'));
</script>

<!-- This is using a single Story with multiple examples to reduce the amount of snapshots generated for visual regression testing -->
<Story name="Default" asChild>
  <h1>Basic Reverse Dependency</h1>
  <Row
    dependency={createDependency()}
    descriptionPromise={resolved('Actix Web is a powerful, pragmatic, and fast web framework for Rust')}
  />

  <h1>Different Requirement</h1>
  <Row
    dependency={createDependency({ dependentCrateName: 'hyper', req: '>= 0.14' })}
    descriptionPromise={resolved('A fast and correct HTTP implementation')}
  />

  <h1>No Description</h1>
  <Row dependency={createDependency({ dependentCrateName: 'my-crate' })} descriptionPromise={resolved(null)} />

  <h1>Loading Description</h1>
  <Row dependency={createDependency({ dependentCrateName: 'loading-crate' })} descriptionPromise={pending()} />

  <h1>Failed Description</h1>
  <Row dependency={createDependency({ dependentCrateName: 'error-crate' })} descriptionPromise={rejected()} />

  <h1>High Download Count</h1>
  <Row
    dependency={createDependency({ dependentCrateName: 'serde_json', downloads: 98765432 })}
    descriptionPromise={resolved('A JSON serialization file format')}
  />
</Story>

<style>
  h1 {
    font-size: 0.875rem;
    font-weight: normal;
    opacity: 0.2;
    margin: 1rem 0 0.25rem;
  }
</style>
