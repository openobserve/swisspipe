import { Page, expect } from '@playwright/test';

/**
 * Utility helpers for Workflow Designer E2E tests
 */

export class WorkflowTestHelper {
  constructor(private page: Page) {}

  /**
   * Navigate to workflows list (assumes authentication is already handled by setup)
   */
  async gotoWorkflowDesigner() {
    // Go to workflows list - authentication should be handled by setup
    await this.page.goto('/workflows');

    // Wait for workflows page to load
    await this.page.waitForSelector('h1:has-text("SwissPipe")', { timeout: 10000 });
    await this.page.waitForSelector('button:has-text("Create Workflow")', { timeout: 10000 });
  }

  /**
   * Create a new workflow with given name
   */
  async createWorkflow(name: string) {
    try {
      // Look for "Create Workflow" button
      await this.page.click('button:has-text("Create Workflow")', { timeout: 5000 });

      // Wait for modal to appear
      await this.page.waitForSelector('.fixed.inset-0', { timeout: 5000 });

      // Fill workflow name - wait for input and fill it
      const nameInput = this.page.locator('.fixed.inset-0 input[type="text"]');
      await nameInput.waitFor({ state: 'visible' });
      await nameInput.fill(name);

      // Wait for button to be enabled and not disabled
      const createButton = this.page.locator('.fixed.inset-0 button[type="submit"]:has-text("Create")');
      await createButton.waitFor({ state: 'visible' });

      // Wait until button is enabled (button should not have disabled attribute)
      await this.page.waitForFunction(() => {
        const button = document.querySelector('.fixed.inset-0 button[type="submit"]');
        return button && !button.hasAttribute('disabled');
      }, {}, { timeout: 5000 });

      await createButton.click();

      // Wait for designer to load - should redirect to /workflows/{id}
      await this.page.waitForURL(/\/workflows\/[a-f0-9-]+/, { timeout: 10000 });
      await this.page.waitForSelector('.vue-flow', { timeout: 10000 });
    } catch (error) {
      console.log('Could not create new workflow:', error);
      // Try to navigate to an existing workflow or handle differently
    }
  }

  /**
   * Add an HTTP node to the workflow canvas
   */
  async addHttpNode() {
    try {
      await this.page.click('button:has-text("Add Node"), [data-testid="add-node"]', { timeout: 3000 });
      await this.page.click('button:has-text("HTTP Request"), [data-testid="http-request"]', { timeout: 3000 });
    } catch {
      console.log('Could not add HTTP node via menu, checking for existing nodes');
    }
  }

  /**
   * Open node properties panel for the first HTTP node found
   */
  async openHttpNodeProperties() {
    // Try to find HTTP node
    const httpNode = this.page.locator('.vue-flow__node:has-text("HTTP"), [data-node-type="http-request"]').first();

    if (await httpNode.count() > 0) {
      await httpNode.dblclick();
    } else {
      // Fallback to any node
      const anyNode = this.page.locator('.vue-flow__node').first();
      await anyNode.dblclick();
    }

    // Wait for properties panel - the actual selector from NodePropertiesPanel.vue
    await this.page.waitForSelector('.fixed.inset-0', { timeout: 5000 });
  }

  /**
   * Configure basic HTTP settings
   */
  async configureHttpBasics(url: string = 'https://httpbin.org/get', method: string = 'GET') {
    const urlInput = this.page.locator('input[placeholder*="https://"], input[type="url"]').first();
    if (await urlInput.isVisible()) {
      await urlInput.fill(url);
    }

    const methodSelect = this.page.locator('select:has(option[value="Get"])');
    if (await methodSelect.isVisible()) {
      await methodSelect.selectOption(method === 'GET' ? 'Get' : method);
    }
  }

  /**
   * Complete setup: navigate, create workflow, add HTTP node, open properties
   */
  async setupHttpWorkflow(workflowName: string = 'Test Workflow') {
    await this.gotoWorkflowDesigner();
    await this.createWorkflow(workflowName);
    await this.addHttpNode();
    await this.openHttpNodeProperties();
    await this.configureHttpBasics();
  }
}

export class HttpLoopTestHelper {
  constructor(private page: Page) {}

  /**
   * Enable the HTTP loop functionality
   */
  async enableLoop() {
    const enableButton = this.page.locator('button:has-text("Disabled")');
    await expect(enableButton).toBeVisible();
    await enableButton.click();
    await expect(this.page.locator('button:has-text("Enabled")')).toBeVisible();
  }

  /**
   * Disable the HTTP loop functionality
   */
  async disableLoop() {
    const disableButton = this.page.locator('button:has-text("Enabled")');
    if (await disableButton.count() > 0) {
      await disableButton.click();
    }
    await expect(this.page.locator('button:has-text("Disabled")')).toBeVisible();
  }

  /**
   * Set loop configuration values
   */
  async setLoopConfig(config: {
    maxIterations?: number;
    interval?: number;
    backoffType?: 'Fixed' | 'Exponential';
    fixedInterval?: number;
    exponential?: { base: number; multiplier: number; max: number };
  }) {
    if (config.maxIterations !== undefined) {
      await this.page.fill('input[placeholder="Unlimited"]', config.maxIterations.toString());
    }

    if (config.interval !== undefined) {
      await this.page.fill('input[type="number"]:near(:text("Interval"))', config.interval.toString());
    }

    if (config.backoffType) {
      const backoffSelect = this.page.locator('select:has(option[value="Fixed"])');
      await backoffSelect.selectOption(config.backoffType);

      if (config.backoffType === 'Fixed' && config.fixedInterval) {
        await this.page.fill('input[type="number"]:near(:text("Fixed Interval"))', config.fixedInterval.toString());
      } else if (config.backoffType === 'Exponential' && config.exponential) {
        const { base, multiplier, max } = config.exponential;
        await this.page.fill('input[type="number"]:near(:text("Base"))', base.toString());
        await this.page.fill('input[type="number"]:near(:text("Multiplier"))', multiplier.toString());
        await this.page.fill('input[type="number"]:near(:text("Max"))', max.toString());
      }
    }
  }

  /**
   * Add a termination condition
   */
  async addTerminationCondition() {
    const addButton = this.page.locator('button:has-text("+ Add Condition")');
    await expect(addButton).toBeVisible();
    await addButton.click();

    // Verify condition was added
    const conditionSelects = this.page.locator('select:has(option[value="ResponseContent"])');
    await expect(conditionSelects.first()).toBeVisible();
  }

  /**
   * Configure a termination condition
   */
  async configureCondition(index: number, config: {
    type?: 'ResponseContent' | 'ResponseStatus' | 'ConsecutiveFailures' | 'TotalTime' | 'Custom';
    expression?: string;
    action?: 'Success' | 'Failure' | 'Stop';
  }) {
    if (config.type) {
      const typeSelect = this.page.locator('select:has(option[value="ResponseContent"])').nth(index);
      await typeSelect.selectOption(config.type);
    }

    if (config.expression) {
      const expressionTextarea = this.page.locator('textarea').nth(index);
      await expressionTextarea.fill(config.expression);
    }

    if (config.action) {
      const actionSelect = this.page.locator('select:has(option[value="Success"])').nth(index);
      await actionSelect.selectOption(config.action);
    }
  }

  /**
   * Remove a termination condition
   */
  async removeCondition(index: number = 0) {
    const removeButton = this.page.locator('button:has-text("Remove")').nth(index);
    await removeButton.click();
  }

  /**
   * Apply a quick setup template
   */
  async applyTemplate(template: 'Customer Onboarding' | 'Health Monitoring' | 'Data Sync') {
    await this.page.click(`button:has-text("${template}")`);

    // Wait for template to be applied
    await this.page.waitForTimeout(500);
  }

  /**
   * Verify loop configuration values
   */
  async verifyLoopConfig(expected: {
    enabled?: boolean;
    maxIterations?: string;
    interval?: string;
    backoffType?: 'Fixed' | 'Exponential';
    conditionCount?: number;
  }) {
    if (expected.enabled !== undefined) {
      const buttonText = expected.enabled ? 'Enabled' : 'Disabled';
      await expect(this.page.locator(`button:has-text("${buttonText}")`)).toBeVisible();
    }

    if (expected.maxIterations) {
      await expect(this.page.locator('input[placeholder="Unlimited"]')).toHaveValue(expected.maxIterations);
    }

    if (expected.interval) {
      await expect(this.page.locator('input[type="number"]:near(:text("Interval"))')).toHaveValue(expected.interval);
    }

    if (expected.backoffType) {
      const backoffSelect = this.page.locator('select:has(option[value="Fixed"])');
      await expect(backoffSelect).toHaveValue(expected.backoffType);
    }

    if (expected.conditionCount !== undefined) {
      await expect(this.page.locator('select:has(option[value="ResponseContent"])')).toHaveCount(expected.conditionCount);
    }
  }
}

/**
 * Wait for element with multiple possible selectors
 */
export async function waitForAnySelector(page: Page, selectors: string[], timeout: number = 5000) {
  const promises = selectors.map(selector =>
    page.waitForSelector(selector, { timeout }).catch(() => null)
  );

  const result = await Promise.race(promises);
  if (!result) {
    throw new Error(`None of the selectors were found: ${selectors.join(', ')}`);
  }

  return result;
}

/**
 * Click on element with multiple possible selectors
 */
export async function clickAnySelector(page: Page, selectors: string[]) {
  for (const selector of selectors) {
    const element = page.locator(selector);
    if (await element.count() > 0) {
      await element.click();
      return true;
    }
  }

  throw new Error(`None of the selectors were clickable: ${selectors.join(', ')}`);
}

/**
 * Assert that at least one of the selectors is visible
 */
export async function expectAnyVisible(page: Page, selectors: string[]) {
  let anyVisible = false;

  for (const selector of selectors) {
    const element = page.locator(selector);
    if (await element.count() > 0 && await element.isVisible()) {
      anyVisible = true;
      break;
    }
  }

  if (!anyVisible) {
    throw new Error(`None of the selectors were visible: ${selectors.join(', ')}`);
  }
}

export { expect } from '@playwright/test';