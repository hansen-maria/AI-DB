<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { RouterLink, RouterView, useRoute } from 'vue-router'

const isDarkMode = ref(false)
const route = useRoute()
const mobileMenuOpen = ref(false)
const alertDismissed = ref(false)

const activeAlert = {
  id: 'psos-downtime-2026-04',
  type: 'warning' as 'warning' | 'info' | 'error',
  message: 'Scheduled maintenance: The Psos service may be unavailable starting Thursday (2nd April 2026) due to cluster maintenance. Jobs submitted before then will not be affected.',
}

const dismissAlert = () => {
  alertDismissed.value = true
  localStorage.setItem(`alert-dismissed-${activeAlert.id}`, 'true')
}

const toggleTheme = () => {
  isDarkMode.value = !isDarkMode.value
  updateTheme()
  localStorage.setItem('theme', isDarkMode.value ? 'dark' : 'light')
}

const updateTheme = () => {
  if (isDarkMode.value) {
    document.documentElement.classList.add('dark')
  } else {
    document.documentElement.classList.remove('dark')
  }
}

const closeMobileMenu = () => {
  mobileMenuOpen.value = false
}

onMounted(() => {
  const savedTheme = localStorage.getItem('theme')
  if (savedTheme) {
    isDarkMode.value = savedTheme === 'dark'
  } else {
    isDarkMode.value = window.matchMedia('(prefers-color-scheme: dark)').matches
  }
  updateTheme()

  if (localStorage.getItem(`alert-dismissed-${activeAlert.id}`) === 'true') {
    alertDismissed.value = true
  }
})
</script>

<template>
  <div class="app-container">
    <!-- Alert Banner -->
    <div v-if="!alertDismissed" :class="['alert-banner', `alert-banner--${activeAlert.type}`]">
      <div class="alert-banner__content">
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none"
             stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="alert-banner__icon">
          <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
          <line x1="12" y1="9" x2="12" y2="13"/>
          <line x1="12" y1="17" x2="12.01" y2="17"/>
        </svg>
        <span>{{ activeAlert.message }}</span>
      </div>
      <button class="alert-banner__close" @click="dismissAlert" aria-label="Dismiss notification">
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none"
             stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="18" y1="6" x2="6" y2="18"/>
          <line x1="6" y1="6" x2="18" y2="18"/>
        </svg>
      </button>
    </div>

    <!-- Global Header -->
    <header class="global-header">
      <div class="header-content">
        <RouterLink to="/" class="logo" @click="closeMobileMenu">
          <div class="logo-image">
            <img v-if="isDarkMode" src="./assets/logo-dark.png" alt="AI-DB Logo" class="logo-img" />
            <img v-else src="./assets/logo-light.png" alt="AI-DB Logo" class="logo-img" />
          </div>
          <div class="logo-text">
            <h1>AI-DB</h1>
            <span class="tagline">Already Identified Database</span>
          </div>
        </RouterLink>

        <!-- Desktop Navigation -->
        <nav class="nav-desktop">
          <RouterLink to="/" :class="{ active: route.name === 'home' }">Home</RouterLink>
          <RouterLink to="/submit" :class="{ active: route.name === 'submit' }">Submit Job</RouterLink>
          <RouterLink to="/jobs" :class="{ active: route.name === 'jobs' }">Jobs</RouterLink>
          <a href="/api/docs" target="_blank" rel="noopener">API Docs</a>
          <RouterLink to="/contact" :class="{ active: route.name === 'contact' }">Contact</RouterLink>
        </nav>

        <div class="header-actions">
          <button @click="toggleTheme" class="theme-toggle" :aria-label="isDarkMode ? 'Switch to light mode' : 'Switch to dark mode'">
            <svg v-if="!isDarkMode" xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="12" cy="12" r="5"/>
              <line x1="12" y1="1" x2="12" y2="3"/>
              <line x1="12" y1="21" x2="12" y2="23"/>
              <line x1="4.22" y1="4.22" x2="5.64" y2="5.64"/>
              <line x1="18.36" y1="18.36" x2="19.78" y2="19.78"/>
              <line x1="1" y1="12" x2="3" y2="12"/>
              <line x1="21" y1="12" x2="23" y2="12"/>
              <line x1="4.22" y1="19.78" x2="5.64" y2="18.36"/>
              <line x1="18.36" y1="5.64" x2="19.78" y2="4.22"/>
            </svg>
            <svg v-else xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"/>
            </svg>
          </button>

          <!-- Mobile Menu Button -->
          <button @click="mobileMenuOpen = !mobileMenuOpen" class="mobile-menu-btn">
            <svg v-if="!mobileMenuOpen" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="3" y1="12" x2="21" y2="12"/>
              <line x1="3" y1="6" x2="21" y2="6"/>
              <line x1="3" y1="18" x2="21" y2="18"/>
            </svg>
            <svg v-else xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18"/>
              <line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        </div>
      </div>

      <!-- Mobile Navigation -->
      <nav v-if="mobileMenuOpen" class="nav-mobile">
        <RouterLink to="/" @click="closeMobileMenu" :class="{ active: route.name === 'home' }">Home</RouterLink>
        <RouterLink to="/submit" @click="closeMobileMenu" :class="{ active: route.name === 'submit' }">Submit Job</RouterLink>
        <RouterLink to="/jobs" @click="closeMobileMenu" :class="{ active: route.name === 'jobs' }">Jobs</RouterLink>
        <a href="/api/docs" target="_blank" rel="noopener" @click="closeMobileMenu">API Docs</a>
        <RouterLink to="/contact" @click="closeMobileMenu" :class="{ active: route.name === 'contact' }">Contact</RouterLink>
      </nav>
    </header>

    <!-- Main Content -->
    <main class="main-content">
      <RouterView />
    </main>

    <!-- Footer -->
    <footer class="global-footer">
      <p>&copy; 2026 AI-DB Project. Built for the genomics research community.</p>
    </footer>
  </div>
</template>

<style scoped>
.app-container {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}

/* Alert Banner */
.alert-banner {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
  padding: 0.6rem 2rem;
  font-size: 0.875rem;
  line-height: 1.5;
}

.alert-banner--warning {
  background: hsla(38, 100%, 50%, 0.12);
  border-bottom: 1px solid hsla(38, 100%, 50%, 0.35);
  color: hsl(32, 80%, 35%);
}

.alert-banner--info {
  background: hsla(210, 100%, 56%, 0.1);
  border-bottom: 1px solid hsla(210, 100%, 56%, 0.3);
  color: hsl(210, 70%, 35%);
}

.alert-banner--error {
  background: hsla(0, 80%, 55%, 0.1);
  border-bottom: 1px solid hsla(0, 80%, 55%, 0.3);
  color: hsl(0, 70%, 40%);
}

html.dark .alert-banner--warning {
  color: hsl(38, 100%, 70%);
}

html.dark .alert-banner--info {
  color: hsl(210, 100%, 75%);
}

html.dark .alert-banner--error {
  color: hsl(0, 90%, 72%);
}

.alert-banner__content {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  flex: 1;
  max-width: 1200px;
  margin: 0 auto;
}

.alert-banner__icon {
  flex-shrink: 0;
}

.alert-banner__close {
  flex-shrink: 0;
  background: none;
  border: none;
  cursor: pointer;
  color: inherit;
  opacity: 0.6;
  padding: 0.25rem;
  border-radius: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: opacity 0.2s;
}

.alert-banner__close:hover {
  opacity: 1;
}

/* Global Header */
.global-header {
  border-bottom: 1px solid var(--color-border);
  background: var(--color-background);
  position: sticky;
  top: 0;
  z-index: 100;
}

.header-content {
  max-width: 1200px;
  margin: 0 auto;
  padding: 1rem 2rem;
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 2rem;
}

.logo {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  text-decoration: none;
  color: inherit;
}

.logo-img {
  width: 50px;
  height: 50px;
  object-fit: contain;
  border-radius: 6px;
}

.logo-text {
  display: flex;
  flex-direction: column;
}

.logo h1 {
  font-size: 1.25rem;
  font-weight: 600;
  margin: 0;
  color: var(--color-heading);
  line-height: 1.2;
}

.tagline {
  font-size: 0.75rem;
  color: var(--color-text);
  opacity: 0.7;
}

/* Desktop Navigation */
.nav-desktop {
  display: flex;
  gap: 0.5rem;
}

.nav-desktop a {
  padding: 0.5rem 1rem;
  text-decoration: none;
  color: var(--color-text);
  border-radius: 6px;
  transition: all 0.2s;
  font-size: 0.95rem;
}

.nav-desktop a:hover {
  background: var(--color-background-soft);
  color: var(--color-heading);
}

.nav-desktop a.active {
  background: hsla(160, 100%, 37%, 0.1);
  color: hsla(160, 100%, 37%, 1);
}

/* Header Actions */
.header-actions {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.theme-toggle {
  width: 40px;
  height: 40px;
  border-radius: 8px;
  border: 1px solid var(--color-border);
  background: var(--color-background-soft);
  color: var(--color-text);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.3s;
}

.theme-toggle:hover {
  background: var(--color-background-mute);
  border-color: var(--color-border-hover);
}

.mobile-menu-btn {
  display: none;
  width: 40px;
  height: 40px;
  border-radius: 8px;
  border: 1px solid var(--color-border);
  background: var(--color-background-soft);
  color: var(--color-text);
  cursor: pointer;
  align-items: center;
  justify-content: center;
}

/* Mobile Navigation */
.nav-mobile {
  display: none;
  flex-direction: column;
  padding: 0.5rem 2rem 1rem;
  border-top: 1px solid var(--color-border);
}

.nav-mobile a {
  padding: 0.75rem 1rem;
  text-decoration: none;
  color: var(--color-text);
  border-radius: 6px;
  transition: all 0.2s;
}

.nav-mobile a:hover,
.nav-mobile a.active {
  background: var(--color-background-soft);
  color: hsla(160, 100%, 37%, 1);
}

/* Main Content */
.main-content {
  flex: 1;
  max-width: 1200px;
  margin: 0 auto;
  padding: 2rem;
  width: 100%;
  box-sizing: border-box;
}

/* Footer */
.global-footer {
  border-top: 1px solid var(--color-border);
  padding: 2rem;
  text-align: center;
  color: var(--color-text);
  opacity: 0.7;
  font-size: 0.9rem;
}

.global-footer p {
  margin: 0;
}

/* Responsive */
@media (max-width: 768px) {
  .header-content {
    padding: 0.75rem 1rem;
  }

  .nav-desktop {
    display: none;
  }

  .mobile-menu-btn {
    display: flex;
  }

  .nav-mobile {
    display: flex;
  }

  .main-content {
    padding: 1rem;
  }

  .logo h1 {
    font-size: 1.1rem;
  }

  .tagline {
    display: none;
  }

  .logo-img {
    width: 40px;
    height: 40px;
  }

  .alert-banner {
    padding: 0.6rem 1rem;
  }
}
</style>