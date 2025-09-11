// Test case to understand node output passing

// Scenario 1: Simple Chain A → B → C
// Expected behavior:
// - Node A receives initial input: {data: "start"}
// - Node A outputs: {data: "A_processed_start"}
// - Node B receives: {data: "A_processed_start"} 
// - Node B outputs: {data: "B_processed_A_processed_start"}
// - Node C receives: {data: "B_processed_A_processed_start"}

// Scenario 2: Multiple Inputs A → C, B → C
// Expected behavior:
// - Node A outputs: {data: "A_result"}
// - Node B outputs: {data: "B_result"}
// - Node C receives merged input:
//   {
//     "input_0": {data: "A_result"},
//     "input_1": {data: "B_result"}
//   }

// The question: Should Node C ALSO receive:
// Option 1: Just the outputs from A and B (current behavior)
// Option 2: The outputs from A and B + their original inputs
// Option 3: Something else?

// Example for Option 2:
// Node C would receive:
// {
//   "input_0": {
//     "output": {data: "A_result"},
//     "input": {data: "original_input_to_A"}
//   },
//   "input_1": {
//     "output": {data: "B_result"}, 
//     "input": {data: "original_input_to_B"}
//   }
// }

console.log("This is a test to understand the expected node output behavior");