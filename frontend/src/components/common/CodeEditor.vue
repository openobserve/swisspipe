<template>
  <div class="h-full flex flex-col">
    <div class="flex-1" ref="editorContainer"></div>
    <div class="flex items-center justify-between p-4 border-t border-slate-600">
      <div class="flex items-center space-x-4">
        <span class="text-sm text-gray-400">{{ language.toUpperCase() }}</span>
        <span class="text-sm text-gray-400">Line {{ currentLine }}, Column {{ currentColumn }}</span>
      </div>
      <div class="flex items-center space-x-2">
        <button
          @click="formatCode"
          class="text-sm bg-slate-700 hover:bg-slate-600 text-gray-300 px-3 py-1 rounded transition-colors"
        >
          Format
        </button>
        <button
          @click="saveCode"
          class="text-sm bg-primary-600 hover:bg-primary-700 text-white px-3 py-1 rounded transition-colors"
        >
          Save
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch, nextTick } from 'vue'
import * as monaco from 'monaco-editor'

interface Props {
  modelValue: string
  language: string
  readonly?: boolean
}

interface Emits {
  (e: 'update:modelValue', value: string): void
  (e: 'save', value: string): void
}

const props = withDefaults(defineProps<Props>(), {
  language: 'javascript',
  readonly: false
})

const emit = defineEmits<Emits>()

const editorContainer = ref<HTMLElement>()
const currentLine = ref(1)
const currentColumn = ref(1)

let editor: monaco.editor.IStandaloneCodeEditor | null = null

onMounted(async () => {
  await nextTick()
  initializeEditor()
})

onBeforeUnmount(() => {
  if (editor) {
    editor.dispose()
  }
})

watch(() => props.modelValue, (newValue) => {
  if (editor && editor.getValue() !== newValue) {
    editor.setValue(newValue || '')
  }
})

function initializeEditor() {
  if (!editorContainer.value) return

  // Configure Monaco theme for dark mode
  monaco.editor.defineTheme('swisspipe-dark', {
    base: 'vs-dark',
    inherit: true,
    rules: [
      { token: 'comment', foreground: '6b7280', fontStyle: 'italic' },
      { token: 'keyword', foreground: '60a5fa' },
      { token: 'string', foreground: '34d399' },
      { token: 'number', foreground: 'f59e0b' },
      { token: 'regexp', foreground: 'f472b6' }
    ],
    colors: {
      'editor.background': '#1f2937',
      'editor.foreground': '#f3f4f6',
      'editor.selectionBackground': '#374151',
      'editor.lineHighlightBackground': '#374151',
      'editorCursor.foreground': '#60a5fa',
      'editorWhitespace.foreground': '#6b7280'
    }
  })

  // Create editor with full features
  editor = monaco.editor.create(editorContainer.value, {
    value: props.modelValue || '',
    language: props.language,
    theme: 'swisspipe-dark',
    readOnly: props.readonly,
    automaticLayout: true,
    minimap: { enabled: true },
    scrollBeyondLastLine: false,
    fontSize: 14,
    lineNumbers: 'on',
    renderWhitespace: 'selection',
    tabSize: 2,
    insertSpaces: true,
    wordWrap: 'on',
    contextmenu: true,
    quickSuggestions: true,
    suggestOnTriggerCharacters: true,
    acceptSuggestionOnEnter: 'on',
    bracketPairColorization: { enabled: true },
    guides: {
      indentation: true,
      bracketPairs: true
    }
  })

  // Configure JavaScript with minimal TypeScript features to avoid worker issues
  if (props.language === 'javascript') {
    try {
      // Disable TypeScript validation entirely but keep syntax highlighting
      monaco.languages.typescript.javascriptDefaults.setDiagnosticsOptions({
        noSemanticValidation: true,
        noSyntaxValidation: true,
        noSuggestionDiagnostics: true,
      })
      
      // Minimal compiler options
      monaco.languages.typescript.javascriptDefaults.setCompilerOptions({
        allowJs: true,
        checkJs: false,
        noLib: true,
        lib: []
      })
    } catch (error) {
      console.warn('Failed to configure JavaScript language features:', error)
    }
  }

  // Listen for content changes
  editor.onDidChangeModelContent(() => {
    if (editor) {
      const value = editor.getValue()
      emit('update:modelValue', value)
    }
  })

  // Listen for cursor position changes
  editor.onDidChangeCursorPosition((e) => {
    currentLine.value = e.position.lineNumber
    currentColumn.value = e.position.column
  })

  // Add keyboard shortcuts
  editor.addAction({
    id: 'save',
    label: 'Save',
    keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS],
    run: () => saveCode()
  })

  editor.addAction({
    id: 'format',
    label: 'Format Document',
    keybindings: [monaco.KeyMod.Shift | monaco.KeyMod.Alt | monaco.KeyCode.KeyF],
    run: () => formatCode()
  })
}

function formatCode() {
  if (editor) {
    editor.trigger('', 'editor.action.formatDocument', {})
  }
}

function saveCode() {
  if (editor) {
    const value = editor.getValue()
    emit('save', value)
  }
}
</script>

<style scoped>
/* Monaco editor container */
.monaco-editor {
  --vscode-editor-background: #1f2937;
}
</style>