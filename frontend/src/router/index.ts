import { createRouter, createWebHistory } from 'vue-router'
import HomeView from '@/views/HomeView.vue'

const router = createRouter({
    history: createWebHistory(import.meta.env.BASE_URL),
    routes: [
        {
            path: '/',
            name: 'home',
            component: HomeView,
            meta: {
                title: 'AI-DB - Hash-Based Annotation Service'
            }
        },
        {
            path: '/submit',
            name: 'submit',
            component: () => import('@/views/SubmitJobView.vue'),
            meta: {
                title: 'Submit Job - AI-DB'
            }
        },
        {
            path: '/job/:id',
            name: 'job',
            component: () => import('@/views/JobDetailView.vue'),
            meta: {
                title: 'Job Details - AI-DB'
            }
        },
        {
            path: '/jobs',
            name: 'jobs',
            component: () => import('@/views/JobListView.vue'),
            meta: {
                title: 'All Jobs - AI-DB'
            }
        },
        {
            path: '/docs',
            name: 'api-docs',
            beforeEnter() {
                window.location.href = '/api/docs'
            },
            component: HomeView // Placeholder, will redirect
        }
    ],
    scrollBehavior(to, _from, savedPosition) {
        if (savedPosition) {
            return savedPosition
        } else if (to.hash) {
            return { el: to.hash, behavior: 'smooth' }
        } else {
            return { top: 0 }
        }
    }
})

// Update page title
router.beforeEach((to, _from, next) => {
    document.title = (to.meta.title as string) || 'AI-DB'
    next()
})

export default router
