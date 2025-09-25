import { test, expect } from '@playwright/test';
import { WorkflowTestHelper, HttpLoopTestHelper } from './helpers/workflow-helpers';

/**
 * Simplified HTTP Loop tests focusing on the core "Add Condition" functionality
 */

test.describe('HTTP Loop - Add Condition Button Fix', () => {
  let workflowHelper: WorkflowTestHelper;
  let loopHelper: HttpLoopTestHelper;

  test.beforeEach(async ({ page }) => {
    workflowHelper = new WorkflowTestHelper(page);
    loopHelper = new HttpLoopTestHelper(page);
  });

  test('Add Condition button should work after enabling loop', async ({ page }) => {
    // Setup workflow with HTTP node
    await workflowHelper.setupHttpWorkflow('Add Condition Test');

    // Verify Loop Configuration section is present
    await expect(page.locator('text="Loop Configuration"')).toBeVisible();

    // Initially loop should be disabled
    await expect(page.locator('button:has-text("Disabled")')).toBeVisible();

    // Add Condition button should NOT be visible when disabled
    await expect(page.locator('button:has-text("+ Add Condition")')).not.toBeVisible();

    // Enable the loop
    await loopHelper.enableLoop();

    // Now Add Condition button should be visible
    await expect(page.locator('button:has-text("+ Add Condition")')).toBeVisible();

    // Initial state should show "No termination conditions defined"
    await expect(page.locator('text="No termination conditions defined"')).toBeVisible();

    // Click Add Condition button - this is the main test
    await loopHelper.addTerminationCondition();

    // Verify condition was added successfully
    await expect(page.locator('select:has(option[value="ResponseContent"])')).toBeVisible();
    await expect(page.locator('select:has(option[value="Success"])')).toBeVisible();
    await expect(page.locator('textarea[placeholder*="response"]')).toBeVisible();

    // Verify default values
    await expect(page.locator('select:has(option[value="ResponseContent"])')).toHaveValue('ResponseContent');
    await expect(page.locator('textarea[placeholder*="response"]')).toHaveValue('response.data.status === "completed"');

    // Test adding multiple conditions
    await loopHelper.addTerminationCondition();
    await expect(page.locator('select:has(option[value="ResponseContent"])')).toHaveCount(2);

    // Test removing a condition
    await loopHelper.removeCondition(0);
    await expect(page.locator('select:has(option[value="ResponseContent"])')).toHaveCount(1);
  });

  test('should configure different condition types', async () => {
    await workflowHelper.setupHttpWorkflow('Condition Types Test');
    await loopHelper.enableLoop();
    await loopHelper.addTerminationCondition();

    // Test ResponseContent (default)
    await loopHelper.verifyLoopConfig({ conditionCount: 1 });
    await expect(page.locator('textarea').first()).toHaveValue('response.data.status === "completed"');

    // Test ResponseStatus
    await loopHelper.configureCondition(0, { type: 'ResponseStatus' });
    await expect(page.locator('textarea').first()).toHaveValue('status_code === 200');

    // Test ConsecutiveFailures
    await loopHelper.configureCondition(0, { type: 'ConsecutiveFailures' });
    await expect(page.locator('textarea').first()).toHaveValue('count >= 3');

    // Test TotalTime
    await loopHelper.configureCondition(0, { type: 'TotalTime' });
    await expect(page.locator('textarea').first()).toHaveValue('duration_seconds > 3600');

    // Test Custom
    await loopHelper.configureCondition(0, { type: 'Custom' });
    await expect(page.locator('textarea').first()).toHaveValue('your_custom_condition_here');

    // Test custom expression
    await loopHelper.configureCondition(0, { expression: 'response.data.completed === true' });
    await expect(page.locator('textarea').first()).toHaveValue('response.data.completed === true');
  });

  test('should apply and configure quick setup templates', async () => {
    await workflowHelper.setupHttpWorkflow('Templates Test');
    await loopHelper.enableLoop();

    // Test Customer Onboarding template
    await loopHelper.applyTemplate('Customer Onboarding');
    await loopHelper.verifyLoopConfig({
      maxIterations: '72',
      interval: '3600',
      conditionCount: 2
    });

    // Test Health Monitoring template
    await loopHelper.applyTemplate('Health Monitoring');
    await loopHelper.verifyLoopConfig({
      interval: '30',
      backoffType: 'Exponential',
      conditionCount: 2
    });

    // Test Data Sync template
    await loopHelper.applyTemplate('Data Sync');
    await loopHelper.verifyLoopConfig({
      maxIterations: '5',
      interval: '1',
      backoffType: 'Exponential',
      conditionCount: 2
    });
  });

  test('should configure loop settings correctly', async ({ page }) => {
    await workflowHelper.setupHttpWorkflow('Loop Config Test');
    await loopHelper.enableLoop();

    // Test basic loop settings
    await loopHelper.setLoopConfig({
      maxIterations: 10,
      interval: 60
    });

    await loopHelper.verifyLoopConfig({
      maxIterations: '10',
      interval: '60'
    });

    // Test fixed backoff
    await loopHelper.setLoopConfig({
      backoffType: 'Fixed',
      fixedInterval: 120
    });

    await loopHelper.verifyLoopConfig({
      backoffType: 'Fixed'
    });

    await expect(page.locator('input[type="number"]:near(:text("Fixed Interval"))')).toHaveValue('120');

    // Test exponential backoff
    await loopHelper.setLoopConfig({
      backoffType: 'Exponential',
      exponential: { base: 5, multiplier: 2.0, max: 300 }
    });

    await loopHelper.verifyLoopConfig({
      backoffType: 'Exponential'
    });

    await expect(page.locator('input[type="number"]:near(:text("Base"))')).toHaveValue('5');
    await expect(page.locator('input[type="number"]:near(:text("Multiplier"))')).toHaveValue('2');
    await expect(page.locator('input[type="number"]:near(:text("Max"))')).toHaveValue('300');
  });

  test('should maintain configuration after closing and reopening properties', async ({ page }) => {
    await workflowHelper.setupHttpWorkflow('Persistence Test');
    await loopHelper.enableLoop();

    // Configure loop with conditions
    await loopHelper.setLoopConfig({
      maxIterations: 5,
      interval: 30
    });

    await loopHelper.addTerminationCondition();
    await loopHelper.configureCondition(0, {
      type: 'ResponseContent',
      expression: 'response.data.success === true',
      action: 'Success'
    });

    // Save/close properties panel
    const closeButton = page.locator('button[aria-label="Close"], button:has(svg):near(:text("Close")), .close-button');
    if (await closeButton.count() > 0) {
      await closeButton.click();
    }

    // Reopen properties
    await workflowHelper.openHttpNodeProperties();

    // Verify settings persisted
    await loopHelper.verifyLoopConfig({
      enabled: true,
      maxIterations: '5',
      interval: '30',
      conditionCount: 1
    });

    await expect(page.locator('textarea').first()).toHaveValue('response.data.success === true');
  });
});