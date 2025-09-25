import { test, expect } from '@playwright/test';

test.describe('Debug HTTP Loop', () => {
  test('should show HTTP loop configuration when available', async ({ page }) => {
    // Go directly to an existing workflow (use one created in previous tests)
    await page.goto('/workflows');

    // Wait for page to load
    await page.waitForSelector('h1:has-text("SwissPipe")', { timeout: 10000 });
    console.log('Workflows page loaded');

    // Look for any existing workflow and click on it
    const existingWorkflow = page.locator('[class*="cursor-pointer"]').first();
    if (await existingWorkflow.count() > 0) {
      await existingWorkflow.click();
      console.log('Clicked on existing workflow');
    } else {
      // Create a new one if none exist
      await page.click('button:has-text("Create Workflow")');
      await page.waitForSelector('.fixed.inset-0', { timeout: 5000 });
      const nameInput = page.locator('.fixed.inset-0 input[type="text"]');
      await nameInput.fill('HTTP Loop Debug');
      await page.waitForFunction(() => {
        const button = document.querySelector('.fixed.inset-0 button[type="submit"]');
        return button && !button.hasAttribute('disabled');
      });
      await page.click('.fixed.inset-0 button[type="submit"]:has-text("Create")');
      console.log('Created new workflow');
    }

    // Wait for workflow designer to load
    await page.waitForURL(/\/workflows\/[a-f0-9-]+/, { timeout: 10000 });
    await page.waitForSelector('.vue-flow', { timeout: 10000 });
    console.log(`In workflow designer: ${page.url()}`);

    // Look for any existing nodes
    const nodes = page.locator('.vue-flow__node');
    const nodeCount = await nodes.count();
    console.log(`Found ${nodeCount} nodes`);

    if (nodeCount > 0) {
      // Click on the first node
      await nodes.first().dblclick();
      console.log('Double-clicked on first node');

      // Wait for properties panel
      await page.waitForSelector('.fixed.inset-0', { timeout: 10000 });
      console.log('Properties panel opened');

      // Check what type of node it is
      const nodeTitle = await page.locator('.fixed.inset-0 h2').textContent();
      console.log(`Node properties panel title: ${nodeTitle}`);

      // Check if HTTP Request configuration is visible
      const httpConfig = page.locator('text="HTTP Request Configuration"');
      if (await httpConfig.count() > 0) {
        console.log('HTTP Request Configuration section found');

        // Look for Loop Configuration
        const loopConfig = page.locator('text="Loop Configuration"');
        if (await loopConfig.count() > 0) {
          console.log('Loop Configuration section found!');
        } else {
          console.log('Loop Configuration section NOT found - this is the issue');
        }
      } else {
        console.log('This is not an HTTP node - looking for HTTP nodes');
      }

      // Take a screenshot for debugging
      await page.screenshot({ path: 'debug-http-loop.png', fullPage: true });
      console.log('Screenshot saved as debug-http-loop.png');
    } else {
      console.log('No nodes found in workflow - need to add HTTP node');
    }
  });
});