import { test, expect } from '@playwright/test';

test.describe('Debug Workflow Creation', () => {
  test('should create workflow step by step', async ({ page }) => {
    // Navigate to workflows page
    await page.goto('/workflows');

    // Wait for page to load
    await page.waitForSelector('h1:has-text("SwissPipe")', { timeout: 10000 });
    await page.waitForSelector('button:has-text("Create Workflow")', { timeout: 10000 });

    console.log('Workflows page loaded');

    // Click create workflow
    await page.click('button:has-text("Create Workflow")');
    console.log('Clicked Create Workflow button');

    // Wait for modal
    await page.waitForSelector('.fixed.inset-0', { timeout: 5000 });
    console.log('Modal appeared');

    // Check if input is visible
    const nameInput = page.locator('.fixed.inset-0 input[type="text"]');
    await nameInput.waitFor({ state: 'visible' });
    console.log('Name input is visible');

    // Fill name
    await nameInput.fill('Debug Test Workflow');
    console.log('Filled workflow name');

    // Check button state
    const createButton = page.locator('.fixed.inset-0 button[type="submit"]:has-text("Create")');
    const isDisabled = await createButton.getAttribute('disabled');
    console.log(`Button disabled state: ${isDisabled}`);

    // Wait for button to be enabled
    await page.waitForFunction(() => {
      const button = document.querySelector('.fixed.inset-0 button[type="submit"]');
      console.log('Button element:', button);
      console.log('Button disabled:', button?.hasAttribute('disabled'));
      return button && !button.hasAttribute('disabled');
    }, {}, { timeout: 5000 });

    console.log('Button should now be enabled');

    // Try to click
    await createButton.click();
    console.log('Clicked create button');

    // Wait for redirect
    await page.waitForURL(/\/workflows\/[a-f0-9-]+/, { timeout: 10000 });
    console.log(`Redirected to: ${page.url()}`);
  });
});