import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { apiClient } from '../services/api'

export interface User {
  username?: string // For basic auth
  id?: string       // For OAuth
  email?: string    // For OAuth
  name?: string     // For OAuth
  picture?: string  // For OAuth
  session_id?: string // For OAuth
}

export const useAuthStore = defineStore('auth', () => {
  const user = ref<User | null>(null)
  const isAuthenticated = computed(() => user.value !== null)

  const login = async (username: string, password: string): Promise<boolean> => {
    try {
      // Create basic auth header
      const credentials = btoa(`${username}:${password}`)
      
      // Test authentication using the API client
      const isValid = await apiClient.validateCredentials(credentials)

      if (isValid) {
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

  const loginWithGoogle = () => {
    // Redirect to Google OAuth login endpoint
    // In development, use VITE_API_BASE_URL or fallback to localhost:3700
    // In production, use the same origin as the frontend
    const getBaseURL = () => {
      if (import.meta.env.DEV) {
        return import.meta.env.VITE_API_BASE_URL || 'http://localhost:3700'
      }
      return window.location.origin
    }

    const baseUrl = getBaseURL()
    window.location.href = `${baseUrl}/auth/google/login`
  }

  const logout = async () => {
    try {
      // If user has session_id, try to logout via API
      if (user.value?.session_id) {
        await apiClient.logout()
      }
    } catch (error) {
      console.error('Logout error:', error)
    } finally {
      // Always clear local state
      user.value = null
      localStorage.removeItem('auth_credentials')
      localStorage.removeItem('oauth_user')
    }
  }

  const initializeAuth = async () => {
    // Check if we have an active OAuth session first (prioritize OAuth over basic auth)
    try {
      const userInfo = await apiClient.getCurrentUser()
      if (userInfo.success && userInfo.user) {
        const oauthUser = {
          id: userInfo.user.id,
          email: userInfo.user.email,
          name: userInfo.user.name,
          picture: userInfo.user.picture,
          session_id: userInfo.session_id
        }
        user.value = oauthUser
        // Store OAuth user info for API client to detect
        localStorage.setItem('oauth_user', JSON.stringify(oauthUser))
        // Clear any basic auth credentials since OAuth takes priority
        localStorage.removeItem('auth_credentials')
        return
      }
    } catch {
      // No active OAuth session, try basic auth
    }

    // Fallback to stored basic auth credentials
    const credentials = localStorage.getItem('auth_credentials')
    if (credentials) {
      try {
        const decoded = atob(credentials)
        const [username] = decoded.split(':')
        user.value = { username }
      } catch {
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
    loginWithGoogle,
    logout,
    initializeAuth,
    getAuthHeaders
  }
})