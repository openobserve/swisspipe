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
export function shallowEqual<T extends Record<string, unknown>>(obj1: T, obj2: T): boolean {
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
export function emailConfigEqual(config1: unknown, config2: unknown): boolean {
  // Quick reference equality check first
  if (config1 === config2) return true
  if (!config1 || !config2) return false

  // Type guards - both must be objects
  if (typeof config1 !== 'object' || typeof config2 !== 'object') return false

  const obj1 = config1 as Record<string, unknown>
  const obj2 = config2 as Record<string, unknown>
  
  // Compare primitive fields
  const primitiveFields = [
    'smtp_config', 'subject', 'template_type', 'body_template', 
    'text_body_template', 'priority', 'max_queue_wait_minutes',
    'queue_if_rate_limited', 'delivery_receipt', 'read_receipt'
  ]
  
  for (const field of primitiveFields) {
    if (obj1[field] !== obj2[field]) {
      return false
    }
  }
  
  // Compare from object
  const from1 = (typeof obj1.from === 'object' && obj1.from) ? obj1.from as Record<string, unknown> : {}
  const from2 = (typeof obj2.from === 'object' && obj2.from) ? obj2.from as Record<string, unknown> : {}
  if (!shallowEqual(from1, from2)) {
    return false
  }
  
  // Compare recipient arrays
  const recipientFields = ['to', 'cc', 'bcc']
  for (const field of recipientFields) {
    const val1 = obj1[field]
    const val2 = obj2[field]
    const arr1 = Array.isArray(val1) ? val1 : []
    const arr2 = Array.isArray(val2) ? val2 : []

    if (arr1.length !== arr2.length) return false

    for (let i = 0; i < arr1.length; i++) {
      const item1 = (typeof arr1[i] === 'object' && arr1[i]) ? arr1[i] as Record<string, unknown> : {}
      const item2 = (typeof arr2[i] === 'object' && arr2[i]) ? arr2[i] as Record<string, unknown> : {}
      if (!shallowEqual(item1, item2)) {
        return false
      }
    }
  }
  
  return true
}