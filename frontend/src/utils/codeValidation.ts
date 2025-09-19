const DANGEROUS_PATTERNS = [
  /eval\s*\(/,
  /Function\s*\(/,
  /setTimeout\s*\(/,
  /setInterval\s*\(/,
  /document\./,
  /window\./,
  /global\./,
  /process\./,
  /require\s*\(/,
  /import\s*\(/,
  /__proto__/,
  /constructor\s*\./,
  /prototype\s*\./
]

const REQUIRED_FUNCTION_PATTERN = /function\s+transformer\s*\([^)]*\)\s*\{[\s\S]*\}/

export function validateGeneratedCode(code: string): { isValid: boolean; error?: string } {
  if (!code || typeof code !== 'string') {
    return { isValid: false, error: 'No code provided' }
  }

  // Check for dangerous patterns
  for (const pattern of DANGEROUS_PATTERNS) {
    if (pattern.test(code)) {
      return {
        isValid: false,
        error: `Code contains potentially unsafe pattern: ${pattern.source}`
      }
    }
  }

  // Check for required transformer function
  if (!REQUIRED_FUNCTION_PATTERN.test(code)) {
    return {
      isValid: false,
      error: 'Code must contain a valid transformer function'
    }
  }

  // Basic syntax validation (simplified)
  try {
    // This is a basic check - in production you might use a proper JS parser
    new Function(code)
  } catch (syntaxError) {
    return {
      isValid: false,
      error: `Syntax error: ${syntaxError instanceof Error ? syntaxError.message : String(syntaxError)}`
    }
  }

  return { isValid: true }
}

export function extractGeneratedCode(rawCode: string): string {
  if (!rawCode || typeof rawCode !== 'string') {
    throw new Error('No code content received from AI')
  }

  let generatedCode = rawCode.trim()

  // Extract code from markdown blocks first (most common case)
  const markdownMatch = generatedCode.match(/```(?:javascript|js)?\s*\n?([\s\S]*?)\n?```/)
  if (markdownMatch && markdownMatch[1]) {
    generatedCode = markdownMatch[1].trim()
  }

  // Find the transformer function using manual brace matching
  const functionStartMatch = generatedCode.match(/function\s+transformer\s*\([^)]*\)\s*\{/)
  if (functionStartMatch) {
    const startIndex = generatedCode.indexOf(functionStartMatch[0])
    const openBraceIndex = startIndex + functionStartMatch[0].lastIndexOf('{')
    let braceCount = 0
    let functionEnd = -1
    let inString = false
    let stringChar = ''

    // Parse character by character to find matching closing brace
    for (let i = openBraceIndex; i < generatedCode.length; i++) {
      const char = generatedCode[i]
      const prevChar = i > 0 ? generatedCode[i - 1] : ''

      // Handle string literals to avoid counting braces inside strings
      if ((char === '"' || char === "'" || char === '`') && prevChar !== '\\') {
        if (!inString) {
          inString = true
          stringChar = char
        } else if (char === stringChar) {
          inString = false
          stringChar = ''
        }
      }

      // Count braces only outside of string literals
      if (!inString) {
        if (char === '{') {
          braceCount++
        } else if (char === '}') {
          braceCount--
          if (braceCount === 0) {
            functionEnd = i + 1
            break
          }
        }
      }
    }

    if (functionEnd > -1) {
      generatedCode = generatedCode.substring(startIndex, functionEnd).trim()
    }
  } else {
    // Fallback: try to wrap content if no complete function found
    const bodyMatch = generatedCode.match(/\{([\s\S]*)\}$/)
    if (bodyMatch) {
      generatedCode = `function transformer(event) {\n    ${bodyMatch[1].trim()}\n}`
    } else if (!generatedCode.startsWith('function transformer')) {
      generatedCode = `function transformer(event) {\n    ${generatedCode}\n    return event;\n}`
    }
  }

  // Validate the extracted code
  const validation = validateGeneratedCode(generatedCode)
  if (!validation.isValid) {
    throw new Error(validation.error || 'Generated code validation failed')
  }

  return generatedCode
}