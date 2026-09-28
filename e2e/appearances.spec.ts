import { test, expect } from '@playwright/test';
const options = ['IBM','Liquid Glass','Apple','Linear','Notion','Material','GitHub','Terminal','Notebook'];
const ids = ['ibm','liquid-glass','apple','linear','notion','material','github','terminal','notebook'];
for (const width of [400,700,1440]) {
  test(`all appearances, both color modes and keyboard at ${width}px`, async ({ page }) => {
    const errors: string[] = []; page.on('pageerror', e => errors.push(e.message));
    await page.setViewportSize({ width, height: 1000 });
    await page.goto('/tests/ui/?settings');
    await page.getByRole('tab', { name: 'Appearance & behavior' }).click();
    await expect(page.getByRole('radio')).toHaveCount(9);
    for (const mode of ['Light','Dark']) {
      await page.getByRole('button', { name: mode, exact: true }).click();
      for (let i=0; i<options.length; i++) {
        await page.getByRole('radio', { name: options[i], exact: true }).check();
        await expect(page.locator('html')).toHaveAttribute('data-appearance', ids[i]);
        await expect(page.getByRole('button', { name: mode, exact: true })).toHaveAttribute('aria-pressed','true');
        expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
        const preview = page.locator('.appearance-option').filter({ has: page.getByRole('radio',{name: options[i],exact:true}) });
        await preview.scrollIntoViewIfNeeded();
      }
    }
    await page.getByRole('radio', { name: 'IBM', exact: true }).focus();
    await page.keyboard.press('ArrowRight');
    await expect(page.getByRole('radio', { name: 'Liquid Glass', exact: true })).toBeChecked();
    await page.locator('fieldset').scrollIntoViewIfNeeded();
    await page.screenshot({ path: `artifacts/appearance-settings-${width}.png`, fullPage: true });
    expect(errors).toEqual([]);
  });
}
test('saved appearance reloads, remote events apply, system mode tracks OS and IBM clears overrides', async ({ page }) => {
  await page.goto('/tests/ui/?settings');
  await page.getByRole('tab', { name: 'Appearance & behavior' }).click();
  await page.getByRole('radio', { name: 'Material', exact: true }).check();
  await page.getByRole('button', { name: 'System', exact: true }).click();
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('data-appearance','material');
  await page.emulateMedia({colorScheme:'light'});
  await expect(page.locator('html')).toHaveClass('light');
  await page.emulateMedia({colorScheme:'dark'});
  await expect(page.locator('html')).toHaveClass('dark');
  await page.evaluate(() => (window as any).fixture.externalAppearance('apple'));
  await expect(page.locator('html')).toHaveAttribute('data-appearance','apple');
  await page.evaluate(() => (window as any).fixture.externalAppearance('ibm'));
  expect(await page.locator('html').getAttribute('style')).toBe('');
  await page.evaluate(() => (window as any).fixture.externalAppearance('unknown'));
  await expect(page.locator('html')).toHaveAttribute('data-appearance','ibm');
});
test('live answers remain readable in all appearances at 400px', async ({ page }) => {
  await page.setViewportSize({width:400,height:900});
  await page.goto('/tests/ui/?overlay');
  await expect(page.getByText('How does this pipeline work?',{exact:false}).first()).toBeVisible();
  for (const mode of ['light','dark']) for (const id of ids) {
    await page.evaluate(({id,mode}) => {
      (window as any).fixture.config.getState().setAppearance(id);
      (window as any).fixture.config.getState().setTheme(mode);
    }, {id,mode});
    await expect(page.getByText('The workflow checks each change before delivery.',{exact:false})).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    const colors = await page.locator('.answer-content').evaluate(el => {
      const probe = document.createElement('span'); probe.style.color='hsl(var(--foreground))'; el.append(probe);
      const expected = getComputedStyle(probe).color; probe.remove();
      return { actual: getComputedStyle(el).color, expected };
    });
    expect(colors.actual).toBe(colors.expected);
    await page.screenshot({path:`artifacts/appearance-overlay-${id}-${mode}.png`});
  }
});

test('appearance controls fit the live settings dialog at 400px', async ({ page }) => {
  await page.setViewportSize({ width: 400, height: 850 });
  await page.goto('/tests/ui/?settings&modal');
  await page.getByRole('tab', { name: 'Appearance & behavior' }).click();
  await page.getByRole('radio', {name:'Apple', exact:true}).check();
  await expect(page.locator('html')).toHaveAttribute('data-appearance','apple');
  const dialog = page.getByRole('dialog', {name:'Settings',exact:true});
  const bounds = await dialog.boundingBox();
  expect(bounds!.width).toBeLessThanOrEqual(400);
  expect(await dialog.evaluate(el => el.scrollWidth <= el.clientWidth)).toBe(true);
  await page.screenshot({path:'artifacts/appearance-settings-modal-400.png'});
});
