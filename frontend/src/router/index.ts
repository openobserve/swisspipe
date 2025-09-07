import { createRouter, createWebHistory } from 'vue-router'

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: '/',
      redirect: '/workflows'
    },
    {
      path: '/workflows',
      name: 'workflows',
      component: () => import('../views/WorkflowListView.vue')
    },
    {
      path: '/workflows/:id',
      name: 'workflow-designer',
      component: () => import('../views/WorkflowDesignerView.vue'),
      props: true
    }
  ],
})

export default router
