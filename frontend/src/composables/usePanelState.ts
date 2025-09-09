import { ref } from 'vue'

export function usePanelState() {
  const showExecutionsPanel = ref(false)
  const showNodeLibrary = ref(true)

  function toggleExecutionsPanel() {
    showExecutionsPanel.value = !showExecutionsPanel.value
  }

  function closeExecutionsPanel() {
    showExecutionsPanel.value = false
  }

  function toggleNodeLibrary() {
    showNodeLibrary.value = !showNodeLibrary.value
  }

  return {
    showExecutionsPanel,
    showNodeLibrary,
    toggleExecutionsPanel,
    closeExecutionsPanel,
    toggleNodeLibrary
  }
}