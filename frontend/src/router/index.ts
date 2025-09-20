import { createRouter, createWebHistory } from 'vue-router'
import { useAuthStore } from '../stores/auth'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/login',
      name: 'login',
      component: () => import('../views/LoginView.vue')
    },
    {
      path: '/auth/callback',
      name: 'oauth-callback',
      component: () => import('../views/OAuthCallbackView.vue')
    },
    {
      path: '/',
      redirect: '/workflows'
    },
    {
      path: '/workflows',
      name: 'workflows',
      component: () => import('../views/WorkflowListView.vue'),
      meta: { requiresAuth: true }
    },
    {
      path: '/workflows/:id',
      name: 'workflow-designer',
      component: () => import('../views/WorkflowDesignerView.vue'),
      props: true,
      meta: { requiresAuth: true }
    },
    {
      path: '/executions',
      name: 'executions',
      component: () => import('../views/ExecutionsView.vue'),
      meta: { requiresAuth: true }
    },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('../views/SettingsView.vue'),
      meta: { requiresAuth: true }
    },
    {
      path: '/api-docs',
      name: 'api-docs',
      component: () => import('../views/ApiDocumentationView.vue'),
      meta: { requiresAuth: true }
    }
  ],
})

router.beforeEach(async (to, from, next) => {
  const authStore = useAuthStore()

  // Initialize auth on first navigation
  if (!authStore.user) {
    await authStore.initializeAuth()
  }

  // Check if route requires authentication
  if (to.meta.requiresAuth && !authStore.isAuthenticated) {
    next('/login')
  } else if (to.path === '/login' && authStore.isAuthenticated) {
    next('/workflows')
  } else {
    next()
  }
})

export default router
