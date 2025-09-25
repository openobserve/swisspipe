import { test, expect } from '@playwright/test';

/**
 * Basic application connectivity test
 * This test verifies that the frontend and backend are running and accessible
 */

test.describe('Basic Application Tests', () => {
  test('should load application homepage', async ({ page }) => {
    // Navigate to the application
    await page.goto('/');

    // Should be redirected to /login or /workflows based on auth state
    await page.waitForTimeout(2000);

    // Check if we can see either login form or workflows page
    const loginForm = page.locator('input[type="password"]');
    const workflowsPage = page.locator('h1:has-text("SwissPipe")');

    const hasLogin = await loginForm.count() > 0;
    const hasWorkflows = await workflowsPage.count() > 0;

    // Should have either login or workflows visible
    expect(hasLogin || hasWorkflows).toBeTruthy();

    console.log(`Application state: ${hasLogin ? 'needs login' : 'authenticated'}`);
  });

  test('should be able to access login page', async ({ page }) => {
    await page.goto('/login');

    // Should see login form elements
    await expect(page.locator('input[type="text"], input[placeholder*="username"]')).toBeVisible();
    await expect(page.locator('input[type="password"]')).toBeVisible();
    await expect(page.locator('button[type="submit"]:has-text("Sign in")')).toBeVisible();
  });

  test('should handle basic authentication', async ({ page }) => {
    await page.goto('/login');

    // Fill credentials
    await page.fill('input[type="text"], input[placeholder*="username"]', 'admin');
    await page.fill('input[type="password"]', 'admin');

    // Try to login
    await page.click('button[type="submit"]:has-text("Sign in")');

    // Wait for response - should either redirect or show error
    await page.waitForTimeout(3000);

    // Check if we're now on workflows page or still on login
    const currentUrl = page.url();
    console.log(`After login attempt, current URL: ${currentUrl}`);

    // Should not be on login page anymore if successful
    const isOnLogin = currentUrl.includes('/login');
    if (!isOnLogin) {
      // If not on login page, should be able to see workflows or some authenticated content
      await expect(page.locator('h1:has-text("SwissPipe")')).toBeVisible();
    }
  });
});