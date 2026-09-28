import { readFile, stat } from 'node:fs/promises';
import { homedir, platform } from 'node:os';
import { join } from 'node:path';
import { request } from 'node:http';

export function controlPath(os = platform(), env = process.env, home = homedir()) {
  if (env.MEETINGHELPER_CONTROL_FILE) return env.MEETINGHELPER_CONTROL_FILE;
  const base = os === 'darwin' ? join(home, 'Library', 'Application Support')
    : os === 'win32' ? (env.APPDATA || join(home, 'AppData', 'Roaming'))
      : (env.XDG_DATA_HOME || join(home, '.local', 'share'));
  return join(base, 'com.nexq.app', 'zaiqo-control.json');
}

export function safeError(error) {
  const message = error instanceof Error ? error.message : String(error);
  return message.replace(/(?:AIza[\w-]+|AQ\.[\w-]+)/g, '[redacted]')
    .replace(/(Bearer\s+|(?:api[_-]?key|token|key)=)[^\s&"']+/gi, '$1[redacted]')
    .replace(/sk-[\w-]+/g, '[redacted]').slice(0, 1200);
}

export async function callApp(tool, args = {}, { signal, timeoutMs = 180_000 } = {}) {
  let control;
  try {
    const path = controlPath();
    const info = await stat(path);
    if (platform() !== 'win32' && (info.mode & 0o077)) throw new Error('permissions');
    control = JSON.parse(await readFile(path, 'utf8'));
  } catch {
    throw new Error('Open KebabM. Its private control file must exist and be owner-only (0600 on macOS/Linux).');
  }
  if (control.host !== '127.0.0.1' || ![47331, 47332, 47333].includes(control.port)
      || !/^[a-f0-9]{32}$/.test(control.token)) throw new Error('Invalid local control configuration. Restart the desktop app.');
  const combined = AbortSignal.any([AbortSignal.timeout(timeoutMs), ...(signal ? [signal] : [])]);
  let response;
  try {
    // Indexing may take longer than fetch's independent response-header deadline.
    // Keep one explicit operation deadline and close the socket on cancellation.
    response = await new Promise((resolve, reject) => {
      const body = JSON.stringify({ tool, arguments: args });
      const req = request({
        hostname: '127.0.0.1', port: control.port, path: '/tool',
        method: 'POST', signal: combined, agent: false,
        headers: { Authorization: `Bearer ${control.token}`, 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(body) },
      }, res => {
        const chunks = [];
        res.on('data', chunk => chunks.push(chunk));
        res.on('error', reject);
        res.on('end', () => resolve({ status: res.statusCode, body: Buffer.concat(chunks).toString('utf8') }));
      });
      req.on('error', reject);
      req.end(body);
    });
  } catch {
    throw new Error(combined.aborted ? 'Request cancelled or timed out. Read status before retrying a mutation.' : 'Desktop connection failed. Open the app and retry.');
  }
  let value;
  try { value = JSON.parse(response.body); } catch { throw new Error('Desktop returned an invalid response. Restart the app.'); }
  if (response.status < 200 || response.status >= 300 || value.ok !== true) {
    // Never propagate an arbitrary provider response from a secret-bearing call.
    if (Object.keys(args).some(k => /key|token|auth_value/i.test(k))) throw new Error('Credential operation failed. Check provider and connection; secret values were suppressed.');
    throw new Error(safeError(value.error || `Desktop HTTP ${response.status}`));
  }
  return value.result;
}
