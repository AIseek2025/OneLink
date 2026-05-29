import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const BFF_YAML = join(__dirname, '../../../platform/contracts/openapi/bff.yaml');

const API_CLIENT_TS = join(__dirname, '../src/api/client.ts');

function extractBffEndpoints(yamlContent) {
  const pathRegex = /^\s{2}(\/api\/v1\/bff\/\S+):/gm;
  const methodRegex = /^\s{4}(get|post|patch|put|delete):/gm;
  const endpoints = [];
  const lines = yamlContent.split('\n');
  let currentPath = null;
  for (const line of lines) {
    const pathMatch = line.match(/^\s{2}(\/api\/v1\/bff\/\S+):/);
    if (pathMatch) {
      currentPath = pathMatch[1];
      continue;
    }
    if (currentPath) {
      const methodMatch = line.match(/^\s{4}(get|post|patch|put|delete):/);
      if (methodMatch) {
        endpoints.push({ method: methodMatch[1].toUpperCase(), path: currentPath });
      }
    }
  }
  return endpoints;
}

function extractClientFetchPaths(clientContent) {
  const fetchRegex = /fetch\(`([^`]*)`\s*(?:,|\))/g;
  const paths = [];
  let match;
  while ((match = fetchRegex.exec(clientContent)) !== null) {
    paths.push(match[1]);
  }
  const templateRegex = /\$\{[^}]+\}/g;
  const resolved = paths.map(p => {
    let r = p;
    r = r.replace(/\$\{IDENTITY_BASE\}/g, '/api/v1/identity');
    r = r.replace(/\$\{BFF_BASE\}/g, '/api/v1/bff');
    r = r.replace(templateRegex, '{param}');
    return r;
  });
  return resolved;
}

const yamlContent = readFileSync(BFF_YAML, 'utf-8');
const clientContent = readFileSync(API_CLIENT_TS, 'utf-8');

const bffEndpoints = extractBffEndpoints(yamlContent);
const clientPaths = extractClientFetchPaths(clientContent);

console.log('BFF OpenAPI endpoints (frozen):');
for (const ep of bffEndpoints) {
  console.log(`  ${ep.method} ${ep.path}`);
}
console.log(`\nAPI client fetch paths:`);
for (const p of clientPaths) {
  console.log(`  ${p}`);
}

const bffPaths = bffEndpoints.map(ep => ep.path);
const clientBffPaths = clientPaths.filter(p => p.startsWith('/api/v1/bff'));

const missingFromClient = bffPaths.filter(p => {
  const norm = p.replace('{userId}', '{param}');
  return !clientBffPaths.some(cp => cp.startsWith(norm.replace('{param}', '')) || cp === norm);
});
if (missingFromClient.length > 0) {
  console.log(`\nWARNING: BFF endpoints not referenced in API client: ${missingFromClient.join(', ')}`);
}

const missingFromOpenApi = clientBffPaths.filter(cp => {
  return !bffPaths.some(bp => {
    const normBp = bp.replace('{userId}', '{param}');
    return cp.startsWith(normBp.replace('{param}', '')) || cp === normBp;
  });
});

if (missingFromOpenApi.length > 0) {
  console.log(`\nWARNING: Client paths not in BFF OpenAPI: ${missingFromOpenApi.join(', ')}`);
}

const frozenVersionMatch = yamlContent.match(/version:\s*(\S+)/);
const freezeDeclaration = yamlContent.includes('冻结声明') || yamlContent.includes('freeze');
console.log(`\nBFF OpenAPI version: ${frozenVersionMatch ? frozenVersionMatch[1] : 'unknown'}`);
console.log(`Freeze declaration present: ${freezeDeclaration}`);

if (!freezeDeclaration) {
  console.error('FAIL: BFF OpenAPI missing freeze declaration');
  process.exit(1);
}

if (bffEndpoints.length < 7) {
  console.error(`FAIL: Expected at least 7 BFF endpoints, found ${bffEndpoints.length}`);
  process.exit(1);
}

console.log('\nPASS: BFF OpenAPI mock verification complete');
