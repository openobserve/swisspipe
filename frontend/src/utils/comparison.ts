/**
 * Efficient shallow comparison utilities and deep cloning
 */

/**
 * Deep clone an object to prevent reference sharing
 * More efficient than JSON.parse(JSON.stringify()) for known structures
 */
export function deepClone<T>(obj: T): T {
  if (obj === null || typeof obj !== 'object') {
    return obj
  }
  
  if (obj instanceof Date) {
    return new Date(obj.getTime()) as unknown as T
  }
  
  if (Array.isArray(obj)) {
    return obj.map(item => deepClone(item)) as unknown as T
  }
  
  if (typeof obj === 'object') {
    const cloned = {} as T
    for (const key in obj) {
      if (obj.hasOwnProperty(key)) {
        cloned[key] = deepClone(obj[key])
      }
    }
    return cloned
  }
  
  return obj
}

/**
 * Performs shallow comparison of two objects
 * Much faster than JSON.stringify for object comparison
 */
export function shallowEqual<T extends Record<string, any>>(obj1: T, obj2: T): boolean {
  const keys1 = Object.keys(obj1)
  const keys2 = Object.keys(obj2)
  
  if (keys1.length !== keys2.length) {
    return false
  }
  
  for (const key of keys1) {
    if (obj1[key] !== obj2[key]) {
      return false
    }
  }
  
  return true
}

/**
 * Performs shallow comparison of arrays
 */
export function shallowArrayEqual<T>(arr1: T[], arr2: T[]): boolean {
  if (arr1.length !== arr2.length) {
    return false
  }
  
  for (let i = 0; i < arr1.length; i++) {
    if (arr1[i] !== arr2[i]) {
      return false
    }
  }
  
  return true
}

/**
 * Deep comparison only for specific EmailConfig fields that need it
 * More efficient than full JSON.stringify
 */
export function emailConfigEqual(config1: any, config2: any): boolean {
  // Quick reference equality check first
  if (config1 === config2) return true
  if (!config1 || !config2) return false
  
  // Compare primitive fields
  const primitiveFields = [
    'smtp_config', 'subject', 'template_type', 'body_template', 
    'text_body_template', 'priority', 'max_queue_wait_minutes',
    'queue_if_rate_limited', 'delivery_receipt', 'read_receipt'
  ]
  
  for (const field of primitiveFields) {
    if (config1[field] !== config2[field]) {
      return false
    }
  }
  
  // Compare from object
  if (!shallowEqual(config1.from || {}, config2.from || {})) {
    return false
  }
  
  // Compare recipient arrays
  const recipientFields = ['to', 'cc', 'bcc']
  for (const field of recipientFields) {
    const arr1 = config1[field] || []
    const arr2 = config2[field] || []
    
    if (arr1.length !== arr2.length) return false
    
    for (let i = 0; i < arr1.length; i++) {
      if (!shallowEqual(arr1[i] || {}, arr2[i] || {})) {
        return false
      }
    }
  }
  
  return true
}