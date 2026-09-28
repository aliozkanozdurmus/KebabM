import { test, expect } from '@playwright/test';
for (const width of [700, 1440]) {
  test(`prepare, search and review sources at ${width}px`, async ({ page }) => {
    await page.setViewportSize({ width, height: 900 });
    const errors: string[] = []; page.on('pageerror', e => errors.push(e.message));
    await page.goto('/tests/ui/');
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
  await page.getByRole('button', { name: 'New answer', exact: true }).click();
  await expect(page.getByText('The pipeline validates', { exact: false }).first()).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(400);
  expect(errors).toEqual([]);
});
