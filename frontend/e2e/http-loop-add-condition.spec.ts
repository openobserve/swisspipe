import { test, expect } from '@playwright/test';

/**
 * Focused test for HTTP Node Loop "Add Condition" functionality
 * This test specifically addresses the issue where the "+ Add Condition" button wasn't working
 */

test.describe('HTTP Loop Add Condition Button', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to the application
    await page.goto('/');
  });

  test('should successfully add termination condition when loop is enabled', async ({ page }) => {
    // Navigate to workflow designer
    await page.goto('/workflows/designer');

    // Create or select a workflow
    try {
      await page.click('button:has-text("New Workflow")', { timeout: 5000 });
      await page.fill('input[placeholder*="workflow name"], input[type="text"]', 'Test Workflow');
      await page.click('button:has-text("Create")');
    } catch {
      // If no "New Workflow" button, might already be in designer
      console.log('Already in workflow designer or using existing workflow');
    }

    // Wait for workflow designer to load
    await page.waitForSelector('[data-testid="workflow-designer"], .vue-flow', { timeout: 10000 });

    // Add HTTP Request node
    try {
      // Try clicking on a node library/palette
      await page.click('button:has-text("Add Node"), [data-testid="add-node"]', { timeout: 3000 });
      await page.click('button:has-text("HTTP Request"), [data-testid="http-request"]', { timeout: 3000 });
    } catch {
      // Alternative: look for existing nodes on canvas
      const existingNode = await page.locator('.vue-flow__node, [data-node-type="http-request"]').first();
      if (await existingNode.count() > 0) {
        console.log('Using existing HTTP node');
      } else {
        // Try right-click context menu
        await page.click('.vue-flow__pane', { button: 'right', position: { x: 300, y: 200 } });
        await page.click('text="HTTP Request"');
      }
    }

    // Open node properties - try multiple approaches
    const httpNode = page.locator('.vue-flow__node:has-text("HTTP"), [data-node-type="http-request"]').first();

    if (await httpNode.count() > 0) {
      await httpNode.dblclick();
    } else {
      // Try looking for any node and assume it's HTTP
      const anyNode = page.locator('.vue-flow__node').first();
      await anyNode.dblclick();
    }

    // Wait for node properties panel to open
    await page.waitForSelector('[data-testid="node-properties-panel"], .node-properties', { timeout: 5000 });

    // Fill in basic HTTP configuration if inputs are visible
    const urlInput = page.locator('input[placeholder*="https://"], input[type="url"]').first();
    if (await urlInput.isVisible()) {
      await urlInput.fill('https://httpbin.org/get');
    }

    // Look for Loop Configuration section
    await expect(page.locator('text="Loop Configuration"')).toBeVisible();

    // CRITICAL TEST: Enable the loop first
    const enableButton = page.locator('button:has-text("Disabled")');
    await expect(enableButton).toBeVisible();
    await enableButton.click();

    // Verify loop is now enabled
    await expect(page.locator('button:has-text("Enabled")')).toBeVisible();

    // Now the termination conditions section should be visible
    await expect(page.locator('text="Termination Conditions"')).toBeVisible();

    // Check initial state - no conditions
    await expect(page.locator('text="No termination conditions defined"')).toBeVisible();

    // MAIN TEST: Click the "+ Add Condition" button
    const addConditionButton = page.locator('button:has-text("+ Add Condition")');
    await expect(addConditionButton).toBeVisible();

    // This is the critical test - the button should work
    await addConditionButton.click();

    // Verify that a condition was added
    // Look for condition configuration elements
    await expect(page.locator('select:has(option[value="ResponseContent"])').first()).toBeVisible();
    await expect(page.locator('select:has(option[value="Success"])').first()).toBeVisible();
    await expect(page.locator('textarea[placeholder*="response"]').first()).toBeVisible();

    // Verify the default condition values
    await expect(page.locator('select:has(option[value="ResponseContent"])').first()).toHaveValue('ResponseContent');
    await expect(page.locator('select:has(option[value="Success"])').first()).toHaveValue('Success');
    await expect(page.locator('textarea[placeholder*="response"]').first()).toHaveValue('response.data.status === "completed"');

    // Verify we can add multiple conditions
    await addConditionButton.click();

    // Should now have 2 condition blocks
    await expect(page.locator('select:has(option[value="ResponseContent"])')).toHaveCount(2);

    // Test removing a condition
    const removeButton = page.locator('button:has-text("Remove")').first();
    await removeButton.click();

    // Should now have 1 condition block
    await expect(page.locator('select:has(option[value="ResponseContent"])')).toHaveCount(1);
  });

  test('should not show Add Condition button when loop is disabled', async ({ page }) => {
    // Navigate to workflow designer and setup
    await page.goto('/workflows/designer');

    // Similar setup as above but ensure loop is disabled
    await page.waitForSelector('[data-testid="workflow-designer"], .vue-flow', { timeout: 10000 });

    // Assume we have an HTTP node (simplified for this focused test)
    const httpNode = page.locator('.vue-flow__node').first();
    if (await httpNode.count() > 0) {
      await httpNode.dblclick();
    }

    await page.waitForSelector('[data-testid="node-properties-panel"], .node-properties', { timeout: 5000 });

    // Ensure loop is disabled
    const disabledButton = page.locator('button:has-text("Disabled")');
    if (await disabledButton.count() === 0) {
      // If we see "Enabled", click it to disable
      const enabledButton = page.locator('button:has-text("Enabled")');
      if (await enabledButton.count() > 0) {
        await enabledButton.click();
      }
    }

    // Verify loop is disabled
    await expect(page.locator('button:has-text("Disabled")')).toBeVisible();

    // The "+ Add Condition" button should NOT be visible
    await expect(page.locator('button:has-text("+ Add Condition")')).not.toBeVisible();

    // The termination conditions section should not be visible
    await expect(page.locator('text="Max Iterations"')).not.toBeVisible();
  });

  test('should handle condition type changes correctly', async ({ page }) => {
    // Setup similar to first test
    await page.goto('/workflows/designer');
    await page.waitForSelector('[data-testid="workflow-designer"], .vue-flow', { timeout: 10000 });

    const httpNode = page.locator('.vue-flow__node').first();
    if (await httpNode.count() > 0) {
      await httpNode.dblclick();
    }

    await page.waitForSelector('[data-testid="node-properties-panel"], .node-properties', { timeout: 5000 });

    // Enable loop and add condition
    await page.click('button:has-text("Disabled")');
    await page.click('button:has-text("+ Add Condition")');

    // Test changing condition type
    const conditionTypeSelect = page.locator('select:has(option[value="ResponseContent"])').first();

    // Change to ResponseStatus
    await conditionTypeSelect.selectOption('ResponseStatus');
    await expect(conditionTypeSelect).toHaveValue('ResponseStatus');

    // Verify placeholder changes
    const expressionTextarea = page.locator('textarea').first();
    await expect(expressionTextarea).toHaveValue('status_code === 200');

    // Change to ConsecutiveFailures
    await conditionTypeSelect.selectOption('ConsecutiveFailures');
    await expect(expressionTextarea).toHaveValue('count >= 3');

    // Change to TotalTime
    await conditionTypeSelect.selectOption('TotalTime');
    await expect(expressionTextarea).toHaveValue('duration_seconds > 3600');

    // Change to Custom
    await conditionTypeSelect.selectOption('Custom');
    await expect(expressionTextarea).toHaveValue('your_custom_condition_here');
  });

  test('should apply quick setup templates correctly', async ({ page }) => {
    // Setup
    await page.goto('/workflows/designer');
    await page.waitForSelector('[data-testid="workflow-designer"], .vue-flow', { timeout: 10000 });

    const httpNode = page.locator('.vue-flow__node').first();
    if (await httpNode.count() > 0) {
      await httpNode.dblclick();
    }

    await page.waitForSelector('[data-testid="node-properties-panel"], .node-properties', { timeout: 5000 });

    // Enable loop
    await page.click('button:has-text("Disabled")');

    // Test Customer Onboarding template
    await page.click('button:has-text("Customer Onboarding")');

    // Verify template was applied
    const maxIterationsInput = page.locator('input[placeholder="Unlimited"]');
    await expect(maxIterationsInput).toHaveValue('72');

    const intervalInput = page.locator('input[type="number"]:near(:text("Interval"))');
    await expect(intervalInput).toHaveValue('3600');

    // Verify conditions were added
    await expect(page.locator('select:has(option[value="ResponseContent"])')).toHaveCount(2);

    // Test Health Monitoring template
    await page.click('button:has-text("Health Monitoring")');

    // Verify new template settings
    await expect(intervalInput).toHaveValue('30');

    const backoffSelect = page.locator('select:has(option[value="Exponential"])');
    await expect(backoffSelect).toHaveValue('Exponential');

    // Test Data Sync template
    await page.click('button:has-text("Data Sync")');

    // Verify data sync settings
    await expect(maxIterationsInput).toHaveValue('5');
    await expect(intervalInput).toHaveValue('1');
  });
});