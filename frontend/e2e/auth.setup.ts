import { test as setup, expect } from '@playwright/test';

const authFile = 'playwright/.auth/user.json';

setup('authenticate', async ({ page }) => {
  // Go to login page
  await page.goto('/login');

  // Wait for login form
  await page.waitForSelector('input[type="text"], input[type="password"]', { timeout: 10000 });

  // Fill in credentials (default from CLAUDE.md is admin/admin)
  await page.fill('input[placeholder="Username"]', 'admin');
  await page.fill('input[placeholder="Password"]', 'admin');

  // Click login button
  await page.click('button[type="submit"]:has-text("Sign in")');

  // Wait for successful login - should redirect to workflows page
  await page.waitForURL('/workflows', { timeout: 10000 });

  // Verify we're logged in by checking for workflows page elements
  await expect(page.locator('h1:has-text("SwissPipe")')).toBeVisible();

  // Save authentication state
  await page.context().storageState({ path: authFile });
});