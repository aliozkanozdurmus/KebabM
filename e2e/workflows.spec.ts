import { test, expect } from '@playwright/test';
test('semantic quota failure keeps visible saved progress and a resume action', async ({ page }) => {
  await page.goto('/tests/ui/?semantic');
  await page.getByRole('button', { name: 'Projects', exact: true }).click();
  await expect(page.getByText('Semantic index incomplete', { exact: false })).toContainText('32 / 740');
  await page.getByText('Search provider', { exact: true }).click();
  await page.getByRole('button', { name: 'Resume semantic indexing', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('HTTP 429');
  await expect(page.getByText('Semantic index incomplete', { exact: false })).toContainText('48 / 740');
  await expect(page.getByRole('button', { name: 'Resume semantic indexing', exact: true })).toBeEnabled();
});
for (const width of [700, 1440]) {
  test(`prepare, search and review sources at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const errors: string[] = []; page.on('pageerror', e => errors.push(e.message));
    await page.goto('/tests/ui/');
    await page.getByRole('button', { name: 'Projects', exact: true }).click();
    await expect(page.getByRole('heading', { name: 'Crosswalk', exact: true })).toBeVisible();
    await expect(page.getByText('Local sources changed.', { exact: false })).toBeVisible();
    await page.getByRole('textbox', { name: 'Search project sources' }).fill('pipeline');
    await page.getByRole('button', { name: 'Search sources', exact: true }).click();
    await page.getByRole('button', { name: '.github/workflows/ci.yml:1–12', exact: true }).click();
    await expect(page.getByRole('dialog', { name: 'Source snapshot' })).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.getByRole('dialog')).toBeHidden();
    await page.getByRole('button', { name: 'Keep as open question' }).click();
    await expect(page.getByRole('heading', { name: 'Open questions · 1' })).toBeVisible();
    await page.evaluate(() => (window as any).fixture.fail(true));
    await page.getByRole('region', { name: 'Index coverage' }).getByRole('button', { name: 'Update index', exact: true }).click();
    await expect(page.getByRole('alert')).toContainText('previous snapshot');
    await expect(page.getByText('132 files', { exact: false })).toBeVisible();
    await page.screenshot({ path: `artifacts/launcher-${width}.png`, fullPage: true });
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
    expect(errors).toEqual([]);
  });
}
test('400px answer stays readable; source, next answer, error and retry', async ({ page }) => {
  await page.setViewportSize({ width: 400, height: 850 });
  const errors: string[] = []; page.on('pageerror', e => errors.push(e.message));
  await page.goto('/tests/ui/?overlay');
  await expect(page.getByText('The workflow checks each change', { exact: false })).toBeVisible();
  await page.evaluate(() => (window as any).fixture.nextAnswer());
  await expect(page.getByText('The workflow checks each change', { exact: false })).toBeVisible();
  await page.getByText('Sources · 1', { exact: true }).click();
  await page.getByRole('button', { name: /\[S1\] .github\/workflows\/ci.yml/ }).click();
  await expect(page.getByText('Indexed snapshot · line 1')).toBeVisible();
  await page.screenshot({ path: 'artifacts/overlay-400.png', fullPage: true });
  await page.getByRole('button', { name: 'New answer', exact: true }).click();
  await expect(page.getByText('A new answer waits until you choose it.', { exact: true })).toBeVisible();
  await page.evaluate(() => (window as any).fixture.fail(true));
  await page.getByRole('button', { name: 'Shorter', exact: true }).click();
  await expect(page.getByRole('alert')).toContainText('Rate limit');
  await page.evaluate(() => (window as any).fixture.fail(false));
  await page.getByRole('button', { name: 'Retry', exact: true }).click();
  await expect(page.getByRole('alert')).toBeHidden();
  await expect(page.getByText('The pipeline validates', { exact: false }).first()).toBeVisible();
  await page.getByRole('button', { name: 'Follow-up', exact: true }).click();
  await expect(page.locator('.answer-content')).toContainText('Which checks must pass before delivery?');
  await expect(page.locator('.answer-content')).not.toContainText('The pipeline validates');
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(400);
  expect(errors).toEqual([]);
});

test('explicit shortening reveals its result while automatic answers remain queued', async ({ page }) => {
  await page.goto('/tests/ui/?overlay');
  await expect(page.locator('.answer-content')).toContainText('The workflow checks');
  await page.getByRole('button', { name: 'Shorter', exact: true }).click();
  await expect(page.locator('.answer-content')).toContainText('The pipeline validates');
  await page.evaluate(() => (window as any).fixture.nextAnswer());
  await expect(page.locator('.answer-content')).toContainText('The pipeline validates');
  await expect(page.getByRole('button', { name: 'New answer', exact: true })).toBeVisible();
});

test('toolbar Short uses the selected historical answer and its evidence', async ({ page }) => {
  await page.goto('/tests/ui/?overlay');
  await expect(page.locator('.answer-content')).toContainText('The workflow checks');
  await page.evaluate(() => (window as any).fixture.nextAnswer());
  await page.getByRole('button', { name: 'Short (2)', exact: true }).click();
  await expect(page.locator('.answer-content')).toContainText('The pipeline validates');
  const request = await page.evaluate(() => (window as any).fixture.assistRequests[0]);
  expect(request.customQuestion).toContain('The workflow checks');
  expect(request.customQuestion).not.toContain('A new answer waits');
  expect(request.sourceEvidence[0].id).toBe('source-one');
});


test('answer history explains its search limitation instead of a generic warning', async ({ page }) => {
  await page.goto('/tests/ui/?overlay');
  await expect(page.getByText('Semantic index is not built yet. Build it in project preparation; keyword search is active.', { exact: true })).toBeVisible();
  await expect(page.getByText('semantic search unavailable or not configured', { exact: false })).toHaveCount(0);
});

for (const width of [400, 700, 1440]) {
  test(`calendar agenda and preparation at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    await page.goto('/tests/ui/?calendar');
    await expect(page.getByRole('heading', { name: 'Your meetings' })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Crosswalk delivery review' })).toBeVisible();
    await expect(page.getByRole('region', { name: 'Index coverage' })).toHaveCount(0);
    await page.screenshot({ path: `artifacts/calendar-${width}.png`, fullPage: true });
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(width);
    await page.getByRole('button', { name: 'Prepare Crosswalk delivery review' }).click();
    await expect(page.getByRole('dialog')).toContainText('Crosswalk delivery review');
    await page.getByRole('button', { name: 'Cancel', exact: true }).first().click();
    await page.evaluate(() => (window as any).fixture.fail(true));
    await page.getByRole('button', { name: 'Refresh calendar' }).click();
    await expect(page.getByRole('alert')).toContainText('could not refresh');
    await expect(page.getByText('Showing the last successful refresh.', { exact: false })).toBeVisible();
    await expect(page.getByRole('heading', { name: 'Crosswalk delivery review' })).toBeVisible();
  });
}
test('calendar setup connects, clears secret field, and disconnects locally', async ({ page }) => {
  await page.goto('/tests/ui/');
  await page.getByRole('button', { name: 'Set up Google Calendar' }).click();
  await page.getByLabel('OAuth client ID', { exact: true }).fill('fixture.apps.googleusercontent.com');
  await page.getByLabel('OAuth client secret', { exact: true }).fill('synthetic-fixture-secret');
  await page.getByRole('button', { name: 'Connect with Google' }).click();
  await expect(page.getByRole('heading', { name: 'Crosswalk delivery review' })).toBeVisible();
  await page.getByRole('button', { name: 'Calendar settings' }).click();
  await expect(page.getByLabel('OAuth client secret', { exact: true })).toHaveValue('');
  await page.getByRole('button', { name: 'Disconnect from this device' }).click();
  await expect(page.getByRole('button', { name: 'Set up Google Calendar' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Crosswalk delivery review' })).toHaveCount(0);
});


test('readiness checks the meeting parties instead of the legacy global STT setting', async ({ page }) => {
  await page.goto('/tests/ui/?readiness');
  await page.getByRole('button', { name: 'Test Your transcription', exact: true }).click();
  await expect(page.getByText(/groq whisper · connection checked/)).toBeVisible();
  await page.getByRole('button', { name: 'Test Other participants’ transcription', exact: true }).click();
  await expect(page.getByText('No Deepgram API key configured')).toBeVisible();
  expect(await page.evaluate(() => (window as any).fixture.sttTests)).toEqual(['groq_whisper', 'deepgram']);
  await page.evaluate(() => {
    const store = (window as any).fixture.config;
    const audio = store.getState().meetingAudioConfig;
    store.setState({ meetingAudioConfig: { ...audio, them: { ...audio.them, stt_provider: 'web_speech' } } });
  });
  await page.getByRole('button', { name: 'Test Other participants’ transcription', exact: true }).click();
  await expect(page.getByText(/Web Speech only receives microphone input/)).toBeVisible();
  expect(await page.evaluate(() => (window as any).fixture.sttTests)).toEqual(['groq_whisper', 'deepgram']);
  await page.evaluate(() => {
    const store = (window as any).fixture.config;
    const audio = store.getState().meetingAudioConfig;
    store.setState({ meetingAudioConfig: { ...audio, them: { ...audio.them, stt_provider: 'whisper_cpp', local_model_id: 'small' } } });
  });
  await page.getByRole('button', { name: 'Test Other participants’ transcription', exact: true }).click();
  await expect(page.getByText(/Selected local model is not downloaded/)).toBeVisible();
});

for (const provider of ['groq_whisper', 'deepgram']) {
  test(`readiness discards a pending ${provider} result when settings change`, async ({ page }) => {
    await page.goto('/tests/ui/?readiness');
    await expect(page.getByRole('heading', { name: 'Check before joining' })).toBeVisible();
    await page.evaluate((provider) => {
      const f = (window as any).fixture;
      const audio = f.config.getState().meetingAudioConfig;
      f.config.setState({ meetingAudioConfig: { ...audio, you: { ...audio.you, stt_provider: provider } } });
      f.holdStt();
    }, provider);
    const button = page.getByRole('button', { name: 'Test Your transcription', exact: true });
    await button.click();
    await expect(page.getByText('Testing…', { exact: true })).toBeVisible();
    await page.evaluate(() => {
      const f = (window as any).fixture;
      const audio = f.config.getState().meetingAudioConfig;
      f.config.setState({ meetingAudioConfig: { ...audio, you: { ...audio.you, stt_provider: 'whisper_cpp', local_model_id: 'small' } } });
    });
    await expect(page.getByText('Testing…', { exact: true })).toHaveCount(0);
    await page.evaluate(() => (window as any).fixture.releaseStt());
    await expect(button).toBeEnabled();
    await expect(page.getByRole('status').filter({ hasText: 'Not tested' })).toHaveCount(6);
    await button.click();
    await expect(page.getByText(/Selected local model is not downloaded/)).toBeVisible();
  });
}

for (const error of [false, true]) {
  test(`source response cannot follow the reader to another answer (${error ? 'error' : 'success'})`, async ({ page }) => {
    await page.goto('/tests/ui/?overlay');
    await expect(page.locator('.answer-content')).toContainText('The workflow checks');
    await page.evaluate(() => (window as any).fixture.holdEvidence());
    await page.getByText('Sources · 1', { exact: true }).click();
    await page.getByRole('button', { name: /\[S1\] .github\/workflows\/ci.yml/ }).click();
    await expect(page.getByText('Opening source…', { exact: true })).toBeVisible();
    await page.evaluate(() => (window as any).fixture.nextAnswer());
    await page.getByRole('button', { name: 'New answer', exact: true }).click();
    await page.evaluate(error => (window as any).fixture.releaseEvidence(0, error), error);
    await expect(page.getByText('Opening source…', { exact: true })).toHaveCount(0);
    await expect(page.getByText('Source response 0', { exact: true })).toHaveCount(0);
    await expect(page.getByText('Old source failed', { exact: true })).toHaveCount(0);
  });
}

test('latest source request wins even when earlier requests finish later', async ({ page }) => {
  await page.goto('/tests/ui/?overlay');
  await expect(page.locator('.answer-content')).toContainText('The workflow checks');
    await page.evaluate(() => (window as any).fixture.holdEvidence());
  await page.getByText('Sources · 1', { exact: true }).click();
  const source = page.getByRole('button', { name: /\[S1\] .github\/workflows\/ci.yml/ });
  await source.click();
  await source.click();
  await page.evaluate(() => (window as any).fixture.releaseEvidence(1));
  await expect(page.getByText('Source response 1', { exact: true })).toBeVisible();
  await page.evaluate(() => (window as any).fixture.releaseEvidence(0));
  await expect(page.getByText('Source response 1', { exact: true })).toBeVisible();
  await expect(page.getByText('Source response 0', { exact: true })).toHaveCount(0);
});
