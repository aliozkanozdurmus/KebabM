import { Client } from '@modelcontextprotocol/client';
import { StdioClientTransport } from '@modelcontextprotocol/client/stdio';
import { fileURLToPath } from 'node:url';
import { safeError } from './control-client.mjs';

// One-shot operations for diagnostics/scripts. Keep a persistent MCP connection for jobs.
const client = new Client({ name: 'meetinghelper-cli', version: '1.0.0' });
try {
  await client.connect(new StdioClientTransport({
    command: process.execPath,
    args: [fileURLToPath(new URL('./server.mjs', import.meta.url))],
    env: Object.fromEntries(Object.entries(process.env).filter(([, v]) => v !== undefined)),
  }));
  const [name = 'status', input = '{}'] = process.argv.slice(2);
  if (['start_job', 'get_job', 'cancel_job'].includes(name)) throw new Error('Jobs require a persistent MCP host connection. Call the underlying operation directly from this one-shot CLI.');
  const response = name === '--list' ? await client.listTools()
    : await client.callTool({ name, arguments: JSON.parse(input) }, { timeout: 1_800_000 });
  process.stdout.write(JSON.stringify(response.structuredContent ?? response, null, 2) + '\n');
  if (response.isError) process.exitCode = 1;
} catch (error) {
  process.stderr.write(safeError(error) + '\n');
  process.exitCode = 1;
} finally {
  await client.close();
}
