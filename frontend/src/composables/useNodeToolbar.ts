import { ref } from 'vue'

export interface ToolbarState {
  visible: boolean
  nodeId: string | null
  sourceHandle: string | undefined
  position: { x: number; y: number }
}

export function useNodeToolbar() {
  const toolbarState = ref<ToolbarState>({
    visible: false,
    nodeId: null,
    sourceHandle: undefined,
    position: { x: 0, y: 0 }
  })

  /**
   * Show toolbar at handle position
   */
  function showToolbarAtHandle(nodeId: string, sourceHandle: string | undefined, handleElement: HTMLElement) {
    // Get handle position
    const rect = handleElement.getBoundingClientRect()
    const canvasContainer = handleElement.closest('.vue-flow')

    if (!canvasContainer) return

    const canvasRect = canvasContainer.getBoundingClientRect()

    // Position toolbar below and to the right of the handle
    let x = rect.left - canvasRect.left + rect.width / 2 + 10
    let y = rect.bottom - canvasRect.top + 10

    // Check if there's enough space on the right
    const spaceOnRight = canvasRect.width - x
    if (spaceOnRight < 250) {
      // Position to the left of the handle instead
      x = rect.left - canvasRect.left - 210 // 210px = toolbar width + gap
    }

    // Ensure toolbar doesn't go off screen vertically
    const spaceBelow = canvasRect.height - y
    if (spaceBelow < 200) {
      // Position above the handle instead
      y = rect.top - canvasRect.top - 250 // Position above
    }

    // Final bounds checking
    if (x < 10) x = 10
    if (y < 10) y = 10

    toolbarState.value = {
      visible: true,
      nodeId,
      sourceHandle,
      position: { x, y }
    }
  }

  /**
   * Hide toolbar immediately
   */
  function hideToolbar() {
    toolbarState.value = {
      visible: false,
      nodeId: null,
      sourceHandle: undefined,
      position: { x: 0, y: 0 }
    }
  }

  /**
   * Handle click on a source handle
   */
  function onHandleClick(nodeId: string, sourceHandle: string | undefined, event: MouseEvent) {
    console.log('🎯 Handle clicked:', { nodeId, sourceHandle })
    const handleElement = event.target as HTMLElement

    // If toolbar is already visible for this handle, hide it
    if (toolbarState.value.visible &&
        toolbarState.value.nodeId === nodeId &&
        toolbarState.value.sourceHandle === sourceHandle) {
      console.log('🔴 Hiding toolbar')
      hideToolbar()
    } else {
      // Show toolbar for this handle
      console.log('🟢 Showing toolbar at handle')
      showToolbarAtHandle(nodeId, sourceHandle, handleElement)
    }
  }

  return {
    toolbarState,
    showToolbarAtHandle,
    hideToolbar,
    onHandleClick
  }
}
