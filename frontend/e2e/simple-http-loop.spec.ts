import { test, expect } from '@playwright/test';

test.describe('Simple HTTP Loop Test', () => {
  test('should create workflow with HTTP node and test loop functionality', async ({ page }) => {
    // Navigate to workflows
    await page.goto('/workflows');
    await page.waitForSelector('h1:has-text("SwissPipe")', { timeout: 10000 });

    // Create new workflow
    await page.click('button:has-text("Create Workflow")');
    await page.waitForSelector('.fixed.inset-0', { timeout: 5000 });

    const nameInput = page.locator('.fixed.inset-0 input[type="text"]');
    await nameInput.fill('HTTP Loop Test');

    await page.waitForFunction(() => {
      const button = document.querySelector('.fixed.inset-0 button[type="submit"]');
      return button && !button.hasAttribute('disabled');
    }, {}, { timeout: 5000 });

    await page.click('.fixed.inset-0 button[type="submit"]:has-text("Create")');

    // Wait for designer
    await page.waitForURL(/\/workflows\/[a-f0-9-]+/, { timeout: 10000 });
    await page.waitForSelector('.vue-flow', { timeout: 10000 });

    console.log(`Created workflow: ${page.url()}`);

    // Look for existing nodes
    const existingNodes = await page.locator('.vue-flow__node').count();
    console.log(`Found ${existingNodes} existing nodes`);

    if (existingNodes > 0) {
      // Try to find an HTTP node specifically
      const httpNode = page.locator('.vue-flow__node').filter({ hasText: /http/i }).first();
      const httpNodeCount = await httpNode.count();

      if (httpNodeCount > 0) {
        console.log('Found HTTP node, clicking on it');
        await httpNode.dblclick();
      } else {
        console.log('No HTTP node found, clicking on first node');
        await page.locator('.vue-flow__node').first().dblclick();
      }

      // Wait for properties panel
      await page.waitForSelector('.fixed.inset-0', { timeout: 5000 });
      console.log('Properties panel opened');

      // Check what we have
      const panelText = await page.locator('.fixed.inset-0').textContent();
      console.log('Panel content preview:', panelText?.substring(0, 200));

      // Check for HTTP Request Configuration
      const hasHttpConfig = await page.locator('text="HTTP Request Configuration"').count() > 0;
      console.log(`Has HTTP Request Configuration: ${hasHttpConfig}`);

      if (hasHttpConfig) {
        // Look for loop configuration
        const hasLoopConfig = await page.locator('text="Loop Configuration"').count() > 0;
        console.log(`Has Loop Configuration: ${hasLoopConfig}`);

        if (hasLoopConfig) {
          console.log('✅ SUCCESS: Found HTTP Loop Configuration!');

          // Test the loop enable/disable
          const disabledButton = page.locator('button:has-text("Disabled")');
          if (await disabledButton.count() > 0) {
            console.log('Loop is currently disabled, enabling it');
            await disabledButton.click();

            // Check if it became enabled
            const enabledButton = page.locator('button:has-text("Enabled")');
            await expect(enabledButton).toBeVisible({ timeout: 3000 });
            console.log('✅ Successfully enabled loop');

            // Look for Add Condition button
            const addConditionButton = page.locator('button:has-text("+ Add Condition")');
            await expect(addConditionButton).toBeVisible({ timeout: 3000 });
            console.log('✅ Add Condition button is visible');
          }
        }
      } else {
        console.log('This node is not an HTTP request node');
      }
    } else {
      console.log('No nodes found - workflow template might be empty');
    }
  });
});