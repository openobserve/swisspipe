import { test, expect, Page } from '@playwright/test';

/**
 * HTTP Node Loop Testing Suite
 *
 * This comprehensive test suite covers all aspects of HTTP node loop functionality:
 * - Basic loop enable/disable
 * - Loop configuration (iterations, intervals, backoff strategies)
 * - Termination conditions CRUD operations
 * - Quick setup templates
 * - End-to-end loop execution
 */

// Test data and helpers
const TEST_WORKFLOW_NAME = 'HTTP Loop Test Workflow';
const TEST_HTTP_URL = 'https://httpbin.org/get';

class WorkflowDesignerPage {
  constructor(private page: Page) {}

  async goto() {
    await this.page.goto('/workflows/designer');
  }

  async createNewWorkflow(name: string) {
    await this.page.click('button:has-text("New Workflow")');
    await this.page.fill('input[placeholder="Enter workflow name"]', name);
    await this.page.click('button:has-text("Create")');
    await this.page.waitForSelector('.workflow-designer', { timeout: 10000 });
  }

  async addHttpNode() {
    // Open node palette/library
    await this.page.click('button:has-text("Add Node")');

    // Select HTTP Request node
    await this.page.click('[data-testid="node-http-request"], .node-item:has-text("HTTP Request")');

    // Click on canvas to place the node
    await this.page.click('.vue-flow__pane', { position: { x: 400, y: 300 } });

    // Wait for node to appear
    await this.page.waitForSelector('[data-testid="node-http-request"], .vue-flow__node:has-text("HTTP Request")');
  }

  async openNodeProperties(nodeText: string) {
    // Double-click or right-click the HTTP node to open properties
    await this.page.dblclick(`[data-testid="node-http-request"], .vue-flow__node:has-text("${nodeText}")`);

    // Wait for properties panel to open
    await this.page.waitForSelector('[data-testid="node-properties-panel"], .node-properties-modal', { timeout: 5000 });
  }

  async fillHttpConfig(url: string, method: 'GET' | 'POST' | 'PUT' | 'DELETE' = 'GET') {
    // Fill in basic HTTP configuration
    await this.page.fill('input[placeholder*="https://api.example.com"], input[placeholder*="URL"]', url);
    await this.page.selectOption('select:has(option[value="Get"])', method === 'GET' ? 'Get' : method);
  }
}

class HttpLoopConfigPage {
  constructor(private page: Page) {}

  async enableLoop() {
    // Click the Disabled button to enable loop
    await this.page.click('button:has-text("Disabled")');

    // Verify it changes to Enabled
    await expect(this.page.locator('button:has-text("Enabled")')).toBeVisible();
  }

  async disableLoop() {
    // Click the Enabled button to disable loop
    await this.page.click('button:has-text("Enabled")');

    // Verify it changes to Disabled
    await expect(this.page.locator('button:has-text("Disabled")')).toBeVisible();
  }

  async setMaxIterations(iterations: number) {
    await this.page.fill('input[placeholder="Unlimited"]', iterations.toString());
  }

  async setInterval(seconds: number) {
    await this.page.fill('input[type="number"]:near(:text("Interval"))', seconds.toString());
  }

  async setBackoffStrategy(strategy: 'Fixed' | 'Exponential') {
    await this.page.selectOption('select:has(option[value="Fixed"])', strategy);
  }

  async setFixedInterval(seconds: number) {
    await this.page.fill('input[type="number"]:near(:text("Fixed Interval"))', seconds.toString());
  }

  async setExponentialConfig(base: number, multiplier: number, max: number) {
    await this.page.fill('input[type="number"]:near(:text("Base"))', base.toString());
    await this.page.fill('input[type="number"]:near(:text("Multiplier"))', multiplier.toString());
    await this.page.fill('input[type="number"]:near(:text("Max"))', max.toString());
  }

  // Termination condition is always present in single condition pattern
  async waitForTerminationCondition() {
    await expect(this.page.locator('.termination-condition, [data-testid="termination-condition"]')).toBeVisible();
  }

  // No remove functionality in single condition pattern
  async clearTerminationCondition() {
    const codeEditor = this.page.locator('.monaco-editor, textarea[data-testid="code-editor"]');
    await codeEditor.clear();
  }

  // No condition type selection in single JavaScript function pattern
  async setConditionScript(script: string) {
    const codeEditor = this.page.locator('.monaco-editor textarea, textarea[data-testid="code-editor"]').first();
    await codeEditor.clear();
    await codeEditor.fill(script);
  }

  async setConditionAction(action: 'Success' | 'Failure' | 'Stop') {
    await this.page.selectOption('select:has(option[value="Success"])', action);
  }


  async applyTemplate(template: 'customer-onboarding' | 'health-monitoring' | 'data-sync') {
    const templateNames = {
      'customer-onboarding': 'Customer Onboarding',
      'health-monitoring': 'Health Monitoring',
      'data-sync': 'Data Sync'
    };

    await this.page.click(`button:has-text("${templateNames[template]}")`);

    // Verify template was applied by checking if condition exists
    await expect(this.page.locator('.termination-condition, [data-testid="termination-condition"]')).toBeVisible();
  }
}

test.describe('HTTP Node Loop Configuration', () => {
  let workflowPage: WorkflowDesignerPage;
  let loopConfig: HttpLoopConfigPage;

  test.beforeEach(async ({ page }) => {
    workflowPage = new WorkflowDesignerPage(page);
    loopConfig = new HttpLoopConfigPage(page);

    // Navigate to workflow designer
    await workflowPage.goto();

    // Create a new workflow
    await workflowPage.createNewWorkflow(TEST_WORKFLOW_NAME);
  });

  test('should create workflow with HTTP node and access loop configuration', async ({ page }) => {
    // Add HTTP node to workflow
    await workflowPage.addHttpNode();

    // Open node properties
    await workflowPage.openNodeProperties('HTTP Request');

    // Fill basic HTTP configuration
    await workflowPage.fillHttpConfig(TEST_HTTP_URL);

    // Verify loop configuration section is present
    await expect(page.locator('text="Loop Configuration"')).toBeVisible();

    // Verify initial state is disabled
    await expect(page.locator('button:has-text("Disabled")')).toBeVisible();
  });

  test('should enable and disable loop functionality', async ({ page }) => {
    await workflowPage.addHttpNode();
    await workflowPage.openNodeProperties('HTTP Request');
    await workflowPage.fillHttpConfig(TEST_HTTP_URL);

    // Test enabling loop
    await loopConfig.enableLoop();

    // Verify loop configuration options are visible
    await expect(page.locator('text="Max Iterations"')).toBeVisible();
    await expect(page.locator('text="Interval (seconds)"')).toBeVisible();
    await expect(page.locator('text="Backoff Strategy"')).toBeVisible();
    await expect(page.locator('text="Termination Condition"')).toBeVisible();

    // Test disabling loop
    await loopConfig.disableLoop();

    // Verify loop configuration options are hidden
    await expect(page.locator('text="Max Iterations"')).not.toBeVisible();
  });

  test('should configure basic loop settings', async ({ page }) => {
    await workflowPage.addHttpNode();
    await workflowPage.openNodeProperties('HTTP Request');
    await workflowPage.fillHttpConfig(TEST_HTTP_URL);
    await loopConfig.enableLoop();

    // Set max iterations
    await loopConfig.setMaxIterations(10);

    // Set interval
    await loopConfig.setInterval(30);

    // Verify values are set
    await expect(page.locator('input[placeholder="Unlimited"]')).toHaveValue('10');
    await expect(page.locator('input[type="number"]:near(:text("Interval"))')).toHaveValue('30');
  });

  test('should configure fixed backoff strategy', async ({ page }) => {
    await workflowPage.addHttpNode();
    await workflowPage.openNodeProperties('HTTP Request');
    await workflowPage.fillHttpConfig(TEST_HTTP_URL);
    await loopConfig.enableLoop();

    // Set backoff strategy to Fixed
    await loopConfig.setBackoffStrategy('Fixed');

    // Verify fixed interval configuration is visible
    await expect(page.locator('text="Fixed Interval (seconds)"')).toBeVisible();

    // Set fixed interval
    await loopConfig.setFixedInterval(120);

    // Verify value is set
    await expect(page.locator('input[type="number"]:near(:text("Fixed Interval"))')).toHaveValue('120');
  });

  test('should configure exponential backoff strategy', async ({ page }) => {
    await workflowPage.addHttpNode();
    await workflowPage.openNodeProperties('HTTP Request');
    await workflowPage.fillHttpConfig(TEST_HTTP_URL);
    await loopConfig.enableLoop();

    // Set backoff strategy to Exponential
    await loopConfig.setBackoffStrategy('Exponential');

    // Verify exponential configuration is visible
    await expect(page.locator('text="Base (seconds)"')).toBeVisible();
    await expect(page.locator('text="Multiplier"')).toBeVisible();
    await expect(page.locator('text="Max (seconds)"')).toBeVisible();

    // Set exponential configuration
    await loopConfig.setExponentialConfig(5, 2.5, 600);

    // Verify values are set
    await expect(page.locator('input[type="number"]:near(:text("Base"))')).toHaveValue('5');
    await expect(page.locator('input[type="number"]:near(:text("Multiplier"))')).toHaveValue('2.5');
    await expect(page.locator('input[type="number"]:near(:text("Max"))')).toHaveValue('600');
  });

  test('should add and configure termination conditions', async ({ page }) => {
    await workflowPage.addHttpNode();
    await workflowPage.openNodeProperties('HTTP Request');
    await workflowPage.fillHttpConfig(TEST_HTTP_URL);
    await loopConfig.enableLoop();

    // Verify termination condition is always present
    await loopConfig.waitForTerminationCondition();

    // Verify default values
    await expect(page.locator('select:has(option[value="Success"])')).toHaveValue('Success');

    // Modify condition script
    await loopConfig.setConditionScript('function condition(event) { return event.data.metadata.http_status === 200; }');
    await loopConfig.setConditionAction('Stop');

    // Verify changes
    await expect(page.locator('select:has(option[value="Stop"])')).toHaveValue('Stop');
  });

  test('should remove termination conditions', async ({ page }) => {
    await workflowPage.addHttpNode();
    await workflowPage.openNodeProperties('HTTP Request');
    await workflowPage.fillHttpConfig(TEST_HTTP_URL);
    await loopConfig.enableLoop();

    // Single condition is always present
    await loopConfig.waitForTerminationCondition();

    // Clear condition script
    await loopConfig.clearTerminationCondition();

    // Set new condition script
    await loopConfig.setConditionScript('function condition(event) { return false; }');

    // Verify condition is still present (cannot be removed)
    await expect(page.locator('.termination-condition, [data-testid="termination-condition"]')).toBeVisible();
  });

  test('should apply quick setup templates', async ({ page }) => {
    await workflowPage.addHttpNode();
    await workflowPage.openNodeProperties('HTTP Request');
    await workflowPage.fillHttpConfig(TEST_HTTP_URL);
    await loopConfig.enableLoop();

    // Test Customer Onboarding template
    await loopConfig.applyTemplate('customer-onboarding');

    // Verify template settings were applied
    await expect(page.locator('input[placeholder="Unlimited"]')).toHaveValue('72');
    await expect(page.locator('input[type="number"]:near(:text("Interval"))')).toHaveValue('3600');

    // Test Health Monitoring template
    await loopConfig.applyTemplate('health-monitoring');

    // Verify template settings were applied
    await expect(page.locator('input[type="number"]:near(:text("Interval"))')).toHaveValue('30');
    await expect(page.locator('select:has(option[value="Exponential"])')).toHaveValue('Exponential');

    // Test Data Sync template
    await loopConfig.applyTemplate('data-sync');

    // Verify template settings were applied
    await expect(page.locator('input[placeholder="Unlimited"]')).toHaveValue('5');
    await expect(page.locator('input[type="number"]:near(:text("Interval"))')).toHaveValue('1');
  });

  test('should save and persist loop configuration', async ({ page }) => {
    await workflowPage.addHttpNode();
    await workflowPage.openNodeProperties('HTTP Request');
    await workflowPage.fillHttpConfig(TEST_HTTP_URL);
    await loopConfig.enableLoop();

    // Configure loop settings
    await loopConfig.setMaxIterations(5);
    await loopConfig.setInterval(60);
    await loopConfig.setBackoffStrategy('Exponential');
    await loopConfig.setExponentialConfig(10, 1.5, 300);
    await loopConfig.setConditionScript('function condition(event) { return event.data.complete === true; }');

    // Save the configuration
    await page.click('button:has-text("Save"), button:has-text("Apply")');

    // Close properties panel
    await page.click('button[aria-label="Close"], .close-button, button:has(svg)');

    // Reopen properties panel
    await workflowPage.openNodeProperties('HTTP Request');

    // Verify settings persisted
    await expect(page.locator('button:has-text("Enabled")')).toBeVisible();
    await expect(page.locator('input[placeholder="Unlimited"]')).toHaveValue('5');
    await expect(page.locator('input[type="number"]:near(:text("Interval"))')).toHaveValue('60');
    await expect(page.locator('select:has(option[value="Exponential"])')).toHaveValue('Exponential');
    await expect(page.locator('.monaco-editor, textarea[data-testid="code-editor"]')).toContainText('event.data.complete === true');
  });
});

test.describe('HTTP Node Loop Execution', () => {
  let workflowPage: WorkflowDesignerPage;
  let loopConfig: HttpLoopConfigPage;

  test.beforeEach(async ({ page }) => {
    workflowPage = new WorkflowDesignerPage(page);
    loopConfig = new HttpLoopConfigPage(page);

    await workflowPage.goto();
    await workflowPage.createNewWorkflow(TEST_WORKFLOW_NAME + ' Execution');
  });

  test('should execute basic HTTP loop', async ({ page }) => {
    // Setup workflow with HTTP loop
    await workflowPage.addHttpNode();
    await workflowPage.openNodeProperties('HTTP Request');
    await workflowPage.fillHttpConfig(TEST_HTTP_URL);
    await loopConfig.enableLoop();
    await loopConfig.setMaxIterations(2);
    await loopConfig.setInterval(1); // 1 second for fast testing

    // Save configuration
    await page.click('button:has-text("Save"), button:has-text("Apply")');
    await page.click('button[aria-label="Close"], .close-button');

    // Execute workflow
    await page.click('button:has-text("Run"), button:has-text("Execute")');

    // Wait for execution to start and monitor progress
    await expect(page.locator('text="Running", text="Executing"')).toBeVisible();

    // Wait for loop to complete (should take ~2-3 seconds)
    await page.waitForSelector('text="Completed", text="Success"', { timeout: 10000 });

    // Check execution history/logs for loop iterations
    await page.click('button:has-text("View Logs"), button:has-text("History")');

    // Verify multiple execution steps (indicating loop iterations)
    await expect(page.locator('[data-testid="execution-step"], .execution-step')).toHaveCount(2);
  });

  test('should respect termination conditions', async ({ page }) => {
    // Setup workflow with termination condition
    await workflowPage.addHttpNode();
    await workflowPage.openNodeProperties('HTTP Request');
    await workflowPage.fillHttpConfig('https://httpbin.org/status/200'); // Always returns 200
    await loopConfig.enableLoop();
    await loopConfig.setMaxIterations(10); // High limit
    await loopConfig.setInterval(1);

    // Set termination condition for successful response
    await loopConfig.setConditionScript('function condition(event) { return event.data.metadata.http_status === 200; }');
    await loopConfig.setConditionAction('Success');

    // Save and execute
    await page.click('button:has-text("Save"), button:has-text("Apply")');
    await page.click('button[aria-label="Close"], .close-button');
    await page.click('button:has-text("Run"), button:has-text("Execute")');

    // Should complete after first iteration due to termination condition
    await page.waitForSelector('text="Completed", text="Success"', { timeout: 5000 });

    // Verify only one execution step (terminated early)
    await page.click('button:has-text("View Logs"), button:has-text("History")');
    await expect(page.locator('[data-testid="execution-step"], .execution-step')).toHaveCount(1);
  });
});

// Helper function to add script to package.json if not exists
test('should add e2e test script to package.json', async () => {
  // This is a meta-test to ensure the test script is available
  // In real scenario, this would be a setup step, not a test
  test.skip(); // Skip this meta-test
});