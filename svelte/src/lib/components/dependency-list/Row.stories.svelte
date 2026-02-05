<script module lang="ts">
  import type { components } from '@crates-io/api-client';

  import { defineMeta } from '@storybook/addon-svelte-csf';

  import Row from './Row.svelte';

  type Dependency = components['schemas']['EncodableDependency'];

  const { Story } = defineMeta({
    title: 'dependency-list/Row',
    component: Row,
  });

  function createDependency(overrides: Partial<Dependency> = {}): Dependency {
    return {
      id: 1,
      crate_id: 'serde',
      version_id: 42,
      req: '^1.0',
      kind: 'normal',
      optional: false,
      default_features: true,
      features: [],
      downloads: 100000,
      ...overrides,
    };
  }

  let resolved = (value: string | null) => Promise.resolve(value);
  let pending = () => new Promise<string | null>(() => {});
  let rejected = () => Promise.reject(new Error('failed'));
</script>

<!-- This is using a single Story with multiple examples to reduce the amount of snapshots generated for visual regression testing -->
<Story name="Default" asChild>
  <h1>Basic Dependency</h1>
  <Row dependency={createDependency()} descriptionPromise={resolved('A serialization framework')} />

  <h1>Optional Dependency</h1>
  <Row dependency={createDependency({ optional: true })} descriptionPromise={resolved('An optional crate')} />

  <h1>Wildcard Requirement</h1>
  <Row dependency={createDependency({ req: '*' })} descriptionPromise={resolved('Any version')} />

  <h1>No Description</h1>
  <Row dependency={createDependency()} descriptionPromise={resolved(null)} />

  <h1>Loading Description</h1>
  <Row dependency={createDependency()} descriptionPromise={pending()} />

  <h1>Loading Description (optional)</h1>
  <Row dependency={createDependency({ optional: true })} descriptionPromise={pending()} />

  <h1>Failed Description</h1>
  <Row dependency={createDependency()} descriptionPromise={rejected()} />

  <h1>Extra Features (with defaults)</h1>
  <Row
    dependency={createDependency({ features: ['derive', 'alloc'], default_features: true })}
    descriptionPromise={resolved('A crate with extra features')}
  />

  <h1>Features Only (no defaults)</h1>
  <Row
    dependency={createDependency({ features: ['derive'], default_features: false })}
    descriptionPromise={resolved('A crate with explicit features')}
  />

  <h1>No Default Features</h1>
  <Row dependency={createDependency({ default_features: false })} descriptionPromise={resolved('Defaults disabled')} />

  <h1>Optional with Features</h1>
  <Row
    dependency={createDependency({ optional: true, features: ['std', 'alloc'], default_features: false })}
    descriptionPromise={resolved('Optional with features')}
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
