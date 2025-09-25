# E2E Tests for HTTP Node Loop Functionality

This directory contains comprehensive end-to-end tests for the HTTP node loop functionality using Playwright.

## Test Files

### 1. `http-node-loop.spec.ts`
Comprehensive test suite covering all aspects of HTTP node loop functionality:
- Workflow creation and HTTP node setup
- Loop enable/disable functionality
- Loop configuration (iterations, intervals, backoff strategies)
- Termination conditions CRUD operations
- Quick setup templates
- End-to-end loop execution with real HTTP calls

### 2. `http-loop-add-condition.spec.ts`
Focused tests specifically for the "Add Condition" button functionality:
- Tests the core issue where the button wasn't working
- Verifies button visibility based on loop state
- Tests condition type changes and validation
- Tests template application

### 3. `http-loop-simple.spec.ts`
Simplified, more reliable tests using helper utilities:
- Clean, maintainable test structure
- Focus on core functionality
- Better error handling and resilience

### 4. `helpers/workflow-helpers.ts`
Utility classes and helper functions for test maintenance:
- `WorkflowTestHelper`: Common workflow operations
- `HttpLoopTestHelper`: HTTP loop-specific operations
- Helper functions for element selection and interaction

## Running the Tests

### Prerequisites
1. Ensure the backend server is running on the expected port
2. Ensure the frontend dev server is running (`npm run dev`)

### Commands

```bash
# Run all E2E tests
npm run test:e2e

# Run tests with UI (interactive mode)
npm run test:e2e:ui

# Run tests in headed mode (see browser)
npm run test:e2e:headed

# Run only HTTP loop tests
npm run test:e2e:http-loop

# Run specific test file
npx playwright test http-loop-simple.spec.ts

# Run tests in debug mode
npx playwright test --debug
```

## Test Coverage

### Core Functionality
- ✅ Loop enable/disable toggle
- ✅ Basic loop configuration (max iterations, interval)
- ✅ Backoff strategy configuration (Fixed vs Exponential)
- ✅ Termination condition CRUD operations
- ✅ Quick setup templates
- ✅ Configuration persistence

### UI Interactions
- ✅ Button click handling
- ✅ Form input validation
- ✅ Dynamic UI updates based on state
- ✅ Error handling and user feedback

### Integration Testing
- ✅ Workflow creation and node management
- ✅ Properties panel interaction
- ✅ Real HTTP request execution
- ✅ Loop execution with termination conditions

## Known Issues Addressed

### "Add Condition" Button Not Working
The main issue that these tests address was that the "+ Add Condition" button in the HTTP loop configuration wasn't working. The tests verify:

1. **Root Cause**: Button only visible when loop is enabled
2. **Solution**: User must click "Disabled" → "Enabled" button first
3. **Verification**: Tests ensure button works correctly after enabling

### Test Reliability
The tests use multiple selector strategies to handle:
- Dynamic class names and data attributes
- Loading states and timing issues
- Different browser behaviors
- Responsive layout changes

## Test Architecture

### Page Object Model
Tests use helper classes that encapsulate:
- Element selection logic
- Common interaction patterns
- State verification methods
- Error handling

### Resilient Selectors
Tests use multiple selector strategies:
```typescript
// Multiple possible selectors for the same element
const selectors = [
  '[data-testid="add-condition"]',
  'button:has-text("+ Add Condition")',
  '.add-condition-button'
];
```

### State Management
Tests properly handle:
- Async operations with proper waits
- State transitions (disabled → enabled)
- Form validation and updates
- Browser navigation and reloads

## Debugging Tips

### Test Failures
1. **Element Not Found**: Check if selectors need updating
2. **Timing Issues**: Increase timeouts or add explicit waits
3. **State Issues**: Verify prerequisite steps (e.g., loop enabled)

### Local Development
```bash
# Run with debug mode to step through tests
npx playwright test --debug http-loop-simple.spec.ts

# Generate test code by recording interactions
npx playwright codegen localhost:5173

# View test results
npx playwright show-report
```

### CI/CD Integration
Tests are configured for:
- Parallel execution (when appropriate)
- Retry on failure
- Screenshot/video capture on failure
- HTML reporting

## Contributing

When adding new HTTP loop functionality:
1. Add corresponding test coverage
2. Update helper utilities if needed
3. Ensure tests are resilient and maintainable
4. Document any new test patterns or approaches

## Future Enhancements

Potential areas for additional test coverage:
- Performance testing with large loop counts
- Stress testing with complex conditions
- Integration with other node types
- Mobile/responsive behavior testing
- Accessibility testing