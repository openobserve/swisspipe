/**
 * Development-only debugging utilities
 * These will be stripped out in production builds
 */

const isDevelopment = import.meta.env.DEV

/**
 * Development-only logger that gets stripped in production
 */
export const debugLog = {
  /**
   * Log general debug information
   */
  info(message: string, data?: unknown) {
    if (isDevelopment) {
      console.log(`[DEBUG] ${message}`, data)
    }
  },

  /**
   * Log component-specific events
   */
  component(componentName: string, event: string, data?: unknown) {
    if (isDevelopment) {
      console.log(`[${componentName}] ${event}`, data)
    }
  },

  /**
   * Log data transformation events
   */
  transform(operation: string, input: unknown, output?: unknown) {
    if (isDevelopment) {
      console.log(`[TRANSFORM] ${operation}`, {
        input: this.summarizeData(input),
        output: output ? this.summarizeData(output) : undefined
      })
    }
  },

  /**
   * Log user interaction events
   */
  interaction(element: string, action: string, data?: unknown) {
    if (isDevelopment) {
      console.log(`[INTERACTION] ${element} ${action}`, this.summarizeData(data))
    }
  },

  /**
   * Log errors with context
   */
  error(message: string, error: unknown, context?: unknown) {
    if (isDevelopment) {
      console.error(`[ERROR] ${message}`, {
        error,
        context: this.summarizeData(context)
      })
    }
  },

  /**
   * Summarize large data objects for cleaner logging
   */
  summarizeData(data: unknown): unknown {
    if (!isDevelopment) return undefined
    
    if (data === null || data === undefined) return data
    if (typeof data !== 'object') return data
    
    if (Array.isArray(data)) {
      return {
        type: 'Array',
        length: data.length,
        sample: data.slice(0, 2)
      }
    }
    
    const obj = data as Record<string, unknown>
    const keys = Object.keys(obj)
    if (keys.length > 5) {
      return {
        type: 'Object',
        keys: keys.slice(0, 5).concat(['...']),
        preview: Object.fromEntries(keys.slice(0, 3).map(k => [k, obj[k]]))
      }
    }
    
    return data
  }
}

/**
 * Performance measurement utility
 */
export const debugPerf = {
  timers: new Map<string, number>(),

  start(label: string) {
    if (isDevelopment) {
      this.timers.set(label, performance.now())
    }
  },

  end(label: string) {
    if (isDevelopment) {
      const start = this.timers.get(label)
      if (start) {
        const duration = performance.now() - start
        console.log(`[PERF] ${label}: ${duration.toFixed(2)}ms`)
        this.timers.delete(label)
      }
    }
  },

  measure<T>(label: string, fn: () => T): T {
    if (isDevelopment) {
      this.start(label)
      try {
        return fn()
      } finally {
        this.end(label)
      }
    }
    return fn()
  }
}