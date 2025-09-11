import { defineStore } from 'pinia'
import { ref, computed } from 'vue'

export interface User {
  username: string
}

export const useAuthStore = defineStore('auth', () => {
  const user = ref<User | null>(null)
  const isAuthenticated = computed(() => user.value !== null)

  const login = async (username: string, password: string): Promise<boolean> => {
    try {
      // Create basic auth header
      const credentials = btoa(`${username}:${password}`)
      
      // Test authentication by making a request to the management workflows endpoint
      const baseURL = import.meta.env.VITE_API_BASE_URL || 'http://localhost:3701'
      const response = await fetch(`${baseURL}/workflows`, {
        headers: {
          'Authorization': `Basic ${credentials}`
        }
      })

      if (response.ok) {
        user.value = { username }
        // Store credentials in localStorage for subsequent requests
        localStorage.setItem('auth_credentials', credentials)
        return true
      } else {
        return false
      }
    } catch (error) {
      console.error('Login error:', error)
      return false
    }
  }

  const logout = () => {
    user.value = null
    localStorage.removeItem('auth_credentials')
  }

  const initializeAuth = () => {
    const credentials = localStorage.getItem('auth_credentials')
    if (credentials) {
      // Decode the credentials to get the username
      try {
        const decoded = atob(credentials)
        const [username] = decoded.split(':')
        user.value = { username }
      } catch (error) {
        // Invalid credentials in localStorage, remove them
        localStorage.removeItem('auth_credentials')
      }
    }
  }

  const getAuthHeaders = () => {
    const credentials = localStorage.getItem('auth_credentials')
    return credentials ? { 'Authorization': `Basic ${credentials}` } : {}
  }

  return {
    user,
    isAuthenticated,
    login,
    logout,
    initializeAuth,
    getAuthHeaders
  }
})