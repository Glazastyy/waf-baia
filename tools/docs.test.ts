import { describe, expect, test } from 'bun:test';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

const root = join(import.meta.dir, '..');

describe('documentation', () => {
  test('provides operator guides for start, configuration, components, operations and troubleshooting', () => {
    for (const file of [
      'docs/getting-started.md',
      'docs/configuration.md',
      'docs/components.md',
      'docs/operations.md',
      'docs/troubleshooting.md'
    ]) {
      expect(existsSync(join(root, file))).toBe(true);
    }
  });

  test('README links to existing local markdown documents', () => {
    const readme = readFileSync(join(root, 'README.md'), 'utf8');
    const links = [...readme.matchAll(/\[[^\]]+\]\(([^)]+\.md)\)/g)].map((match) => match[1]);

    expect(links).toContain('docs/getting-started.md');
    expect(links).toContain('docs/configuration.md');
    expect(links).toContain('docs/troubleshooting.md');

    for (const link of links) {
      expect(existsSync(join(root, link))).toBe(true);
    }
  });
});
