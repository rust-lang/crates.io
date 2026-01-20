import { afterEach, describe, expect, it, vi } from 'vitest';

import { getPurl } from './purl';

function stubHost(host: string) {
  vi.stubGlobal('location', { host });
}

describe('getPurl', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('returns PURL unchanged for crates.io host', () => {
    stubHost('crates.io');

    let result = getPurl('serde', '1.0.0');

    expect(result).toBe('pkg:cargo/serde@1.0.0');
  });

  it('adds repository_url parameter for non-crates.io hosts', () => {
    stubHost('staging.crates.io');

    let result = getPurl('serde', '1.0.0');

    expect(result).toBe('pkg:cargo/serde@1.0.0?repository_url=https%3A%2F%2Fstaging.crates.io%2F');
  });

  it('adds repository_url parameter for custom registry hosts', () => {
    stubHost('my-registry.example.com');

    let result = getPurl('my-crate', '2.5.0');

    expect(result).toBe('pkg:cargo/my-crate@2.5.0?repository_url=https%3A%2F%2Fmy-registry.example.com%2F');
  });

  it('properly URL encodes the repository URL with port', () => {
    stubHost('registry.example.com:8080');

    let result = getPurl('test', '1.0.0');

    expect(result).toBe('pkg:cargo/test@1.0.0?repository_url=https%3A%2F%2Fregistry.example.com%3A8080%2F');
  });
});
