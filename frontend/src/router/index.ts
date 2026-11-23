import { createRouter, createWebHistory } from 'vue-router'
import HomeView from '@/views/HomeView.vue'

declare module 'vue-router' {
    interface RouteMeta {
        title?: string
        description?: string
        /** Exclude from search engines (private, short-lived or error pages) */
        noindex?: boolean
    }
}

const SITE_URL = 'https://ai-db.computational.bio'
const DEFAULT_TITLE = 'AI-DB – Rapid Bacterial Protein Identification and Annotation Lookup'
const DEFAULT_DESCRIPTION =
    'AI-DB identifies bacterial proteins in seconds by exact, alignment-free hash lookup of existing annotations from Bakta, BakRep and the community.'

const router = createRouter({
    history: createWebHistory(import.meta.env.BASE_URL),
    routes: [
        {
            path: '/',
            name: 'home',
            component: HomeView,
            meta: {
                title: DEFAULT_TITLE,
                description:
                    'Identify bacterial proteins in seconds by exact, alignment-free hash lookup against Bakta, BakRep and community annotations. Free web service and REST API, no installation.'
            }
        },
        {
            path: '/submit',
            name: 'submit',
            component: () => import('@/views/SubmitJobView.vue'),
            meta: {
                title: 'Submit Protein FASTA – AI-DB',
                description:
                    'Upload or paste bacterial protein sequences in FASTA format and get existing annotations (gene, product, COG, EC, GO) within seconds.'
            }
        },
        {
            path: '/job/:id',
            name: 'job',
            component: () => import('@/views/JobDetailView.vue'),
            meta: {
                title: 'Job Details – AI-DB',
                noindex: true
            }
        },
        {
            path: '/jobs',
            name: 'jobs',
            component: () => import('@/views/JobListView.vue'),
            meta: {
                title: 'My Jobs – AI-DB',
                noindex: true
            }
        },
        {
            path: '/about',
            name: 'about',
            component: () => import('@/views/AboutView.vue'),
            meta: {
                title: 'Help, FAQ, Manual & Citation – AI-DB',
                description:
                    'How to use AI-DB for bacterial protein identification: FAQ, step-by-step manual, REST API and command-line usage, and how to cite AI-DB.'
            }
        },
        {
            path: '/contact',
            name: 'contact',
            component: () => import('@/views/ContactView.vue'),
            meta: {
                title: 'Contact & Support – AI-DB',
                description:
                    'Contact the AI-DB team for support, bug reports and collaboration inquiries. Bioinformatics & Systems Biology, Justus Liebig University Giessen.'
            }
        },
        {
            path: '/imprint',
            name: 'imprint',
            component: () => import('@/views/ImprintView.vue'),
            meta: {
                title: 'Imprint & Privacy Policy – AI-DB',
                description: 'Imprint and privacy policy of the AI-DB web service.'
            }
        },
        {
            path: '/terms',
            name: 'terms',
            component: () => import('@/views/TermsView.vue'),
            meta: {
                title: 'Terms of Service – AI-DB',
                description: 'Terms of service for using the AI-DB web service and REST API.'
            }
        },
        {
            path: '/docs',
            name: 'api-docs',
            beforeEnter() {
                window.location.href = '/api/docs'
            },
            component: HomeView // Placeholder, will redirect
        },
        {
            // Catch-all – must stay the last route
            path: '/:pathMatch(.*)*',
            name: 'not-found',
            component: () => import('@/views/NotFoundView.vue'),
            meta: {
                title: 'Page not found – AI-DB',
                noindex: true
            }
        }
    ],
    scrollBehavior(to, _from, savedPosition) {
        if (savedPosition) {
            return savedPosition
        } else if (to.hash) {
            // Offset for the sticky global header so anchors are not hidden beneath it
            return { el: to.hash, top: 90, behavior: 'smooth' }
        } else {
            return { top: 0 }
        }
    }
})

// ── Head tags (title, description, canonical, Open Graph, robots) ──

/** Create or update a <meta>/<link> tag in <head>. */
function setHeadTag(attr: 'name' | 'property' | 'rel', key: string, value: string) {
    const isLink = attr === 'rel'
    const tag = isLink ? 'link' : 'meta'
    let el = document.head.querySelector<HTMLElement>(`${tag}[${attr}="${key}"]`)
    if (!el) {
        el = document.createElement(tag)
        el.setAttribute(attr, key)
        document.head.appendChild(el)
    }
    el.setAttribute(isLink ? 'href' : 'content', value)
}

router.afterEach((to) => {
    const title = to.meta.title ?? 'AI-DB'
    const description = to.meta.description ?? DEFAULT_DESCRIPTION
    const url = SITE_URL + to.path

    document.title = title
    setHeadTag('name', 'description', description)
    setHeadTag('name', 'robots', to.meta.noindex ? 'noindex, nofollow' : 'index, follow')
    setHeadTag('rel', 'canonical', url)
    setHeadTag('property', 'og:title', title)
    setHeadTag('property', 'og:description', description)
    setHeadTag('property', 'og:url', url)
})

export default router
