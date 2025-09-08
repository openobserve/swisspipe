// Default script templates for workflow nodes

export const DEFAULT_CONDITION_SCRIPT = `function condition(event) {
  // get data form the event
  const data = event.data;

  // Do dome check
  if (data.age > 10) {
    return false;
  } else if (data.age <= 10) {
    return true;
  }
  
  return false; // return a final statement if nothing matches
}`;

export const DEFAULT_TRANSFORMER_SCRIPT = `function transformer(event) {
    // Access and modify the event data
    // actual data will be avaliable in event.data

    // Example modifications:
    // Example: event.data.newField = 'newValue';
    // Example: event.data.processed = true;
    
    return event; // Return modified event or null to drop
}`;