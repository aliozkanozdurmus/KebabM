import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { mkdtemp, writeFile, chmod, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { callApp } from '../control-client.mjs';

test('private loopback transport validates config, suppresses secrets and aborts HTTP', async t => {
  const dir = await mkdtemp(join(tmpdir(), 'meetinghelper-mcp-'));
  const path = join(dir, 'control.json');
  const prior = process.env.MEETINGHELPER_CONTROL_FILE;
  process.env.MEETINGHELPER_CONTROL_FILE = path;
  t.after(async () => {
    if (prior === undefined) delete process.env.MEETINGHELPER_CONTROL_FILE;
    else process.env.MEETINGHELPER_CONTROL_FILE = prior;
    await rm(dir, { recursive: true, force: true });
  });
  const token = 'a'.repeat(32); // Synthetic fixture, never a real credential.
  let mode = 'success';
  let connected;
  let disconnected;
  const server = createServer((req, res) => {
    assert.equal(req.headers.authorization, `Bearer ${token}`);
    if (mode === 'hang') {
      req.resume();
      res.on('close', () => disconnected?.());
      connected?.();
      return;
    }
    res.setHeader('Content-Type', 'application/json');
    if (mode === 'failure') { res.statusCode = 400; res.end(JSON.stringify({ ok: false, error: 'arbitrary-private-value' })); }
    else res.end(JSON.stringify({ ok: true, result: { connected: true } }));
  });
  let port;
  for (const candidate of [47331, 47332, 47333]) {
    try {
      await new Promise((resolve, reject) => {
        const fail = error => { server.off('listening', ready); reject(error); };
        const ready = () => { server.off('error', fail); resolve(); };
        server.once('error', fail); server.once('listening', ready); server.listen(candidate, '127.0.0.1');
      });
      port = candidate; break;
    } catch (error) { if (error.code !== 'EADDRINUSE') throw error; }
  }
  assert.ok(port, 'A dedicated local fixture port must be available');
  t.after(() => { server.closeAllConnections(); server.close(); });
  const config = { host: '127.0.0.1', port, token };
  await writeFile(path, JSON.stringify(config), { mode: 0o600 });
  assert.deepEqual(await callApp('status'), { connected: true });
  mode = 'failure';
  await assert.rejects(callApp('store_secret', { key: 'arbitrary-private-value' }), error => !error.message.includes('arbitrary-private-value'));
  await writeFile(path, JSON.stringify({ ...config, host: 'example.com' }));
  await assert.rejects(callApp('status'), /Invalid local control/);
  await writeFile(path, JSON.stringify(config));
  if (process.platform !== 'win32') {
    await chmod(path, 0o644);
    await assert.rejects(callApp('status'), /owner-only/);
    await chmod(path, 0o600);
  }
  mode = 'hang';
  const started = new Promise(resolve => { connected = resolve; });
  const closed = new Promise(resolve => { disconnected = resolve; });
  const controller = new AbortController();
  const pending = callApp('embed_project', {}, { signal: controller.signal });
  const rejected = assert.rejects(pending, /cancelled or timed out/);
  await started; controller.abort(); await rejected;
  await Promise.race([closed, new Promise((_, reject) => {
    const timer = setTimeout(() => reject(new Error('HTTP connection stayed open after cancellation')), 2000);
    timer.unref();
  })]);
});
