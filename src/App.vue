<script setup lang="ts">
import { ref, onMounted } from 'vue'

const isDarkMode = ref(false)

const toggleTheme = () => {
  console.log('Toggle theme clicked! Current state:', isDarkMode.value)
  isDarkMode.value = !isDarkMode.value
  console.log('New state:', isDarkMode.value)
  updateTheme()
  localStorage.setItem('theme', isDarkMode.value ? 'dark' : 'light')
}

const updateTheme = () => {
  console.log('Updating theme, isDarkMode:', isDarkMode.value)
  if (isDarkMode.value) {
    document.documentElement.classList.add('dark')
    console.log('Added dark class to:', document.documentElement)
  } else {
    document.documentElement.classList.remove('dark')
    console.log('Removed dark class from:', document.documentElement)
  }
  console.log('Current classes:', document.documentElement.className)
}

onMounted(() => {
  console.log('Component mounted')
  // Check for saved theme preference or default to system preference
  const savedTheme = localStorage.getItem('theme')
  if (savedTheme) {
    isDarkMode.value = savedTheme === 'dark'
    console.log('Loaded theme from localStorage:', savedTheme)
  } else {
    isDarkMode.value = window.matchMedia('(prefers-color-scheme: dark)').matches
    console.log('Using system preference, dark mode:', isDarkMode.value)
  }
  updateTheme()
})
</script>

<template>
  <div class="landing">
    <!-- Header -->
    <header class="header">
      <div class="logo">
        <div class="logo-image">
          <div class="logo-placeholder">
            <span>LOGO</span>
          </div>
        </div>
        <div class="logo-text">
          <h1>AI-DB</h1>
          <span class="tagline">Already Identified Database</span>
        </div>
      </div>
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
    </header>

    <!-- Hero Section -->
    <section class="hero">
      <h2 class="hero-title">Hash-Based Annotation Service for Microbial Sequencing Data</h2>
      <p class="hero-description">
        AI-DB accelerates the analysis of microbial sequencing data while ensuring data privacy
        and sovereignty through cryptographic hash-based annotations.
      </p>
      <div class="cta-buttons">
        <a href="#features" class="btn btn-primary">Get Started</a>
        <a href="#how-it-works" class="btn btn-secondary">Documentation</a>
      </div>
    </section>

    <!-- Key Features -->
    <section id="features" class="section">
      <h3 class="section-title">Key Features</h3>
      <div class="features-grid">
        <div class="feature">
          <h4>Data Privacy & Sovereignty</h4>
          <p>Your sequence data never leaves your infrastructure. MD5 hashes ensure cryptographic irreversibility while enabling powerful annotations.</p>
        </div>
        <div class="feature">
          <h4>Instant Annotations</h4>
          <p>Skip computationally expensive sequence alignments. Hash-based matching delivers annotations in seconds instead of hours.</p>
        </div>
        <div class="feature">
          <h4>Comprehensive Coverage</h4>
          <p>Access functional classifications, database cross-references, and metadata from extensively characterized UniRef protein sequences.</p>
        </div>
        <div class="feature">
          <h4>Fallback Alignment</h4>
          <p>Seamlessly transitions to Diamond alignment for novel sequences without hash matches, ensuring comprehensive annotation coverage.</p>
        </div>
        <div class="feature">
          <h4>Continuously Expanding</h4>
          <p>Newly characterized sequences can be incorporated into the hash database, creating a growing knowledge base.</p>
        </div>
        <div class="feature">
          <h4>Production Ready</h4>
          <p>Built on the trusted Bakta framework, suitable for both individual research and large-scale comparative genomics.</p>
        </div>
      </div>
    </section>

    <!-- How It Works -->
    <section id="how-it-works" class="section section-alt">
      <h3 class="section-title">How It Works</h3>
      <p class="section-intro">
        AI-DB implements a multi-tiered annotation strategy that combines instant hash-based
        retrieval with classical alignment methods.
      </p>
      <div class="workflow">
        <div class="workflow-step">
          <div class="step-number">1</div>
          <div class="step-content">
            <h4>Hash Conversion</h4>
            <p>Predicted coding sequences are converted into MD5 hashes on your local infrastructure.</p>
          </div>
        </div>
        <div class="workflow-step">
          <div class="step-number">2</div>
          <div class="step-content">
            <h4>Database Query</h4>
            <p>Hashes are queried against the comprehensive hash database derived from UniRef clusters.</p>
          </div>
        </div>
        <div class="workflow-step">
          <div class="step-number">3</div>
          <div class="step-content">
            <h4>Instant Retrieval</h4>
            <p>Hash matches enable immediate retrieval of annotations, classifications, and cross-references.</p>
          </div>
        </div>
        <div class="workflow-step">
          <div class="step-number">4</div>
          <div class="step-content">
            <h4>Alignment Fallback</h4>
            <p>Sequences without matches use Diamond alignment against curated reference databases.</p>
          </div>
        </div>
      </div>
    </section>

    <!-- Technical Details -->
    <section class="section">
      <h3 class="section-title">Technical Approach</h3>
      <div class="technical-content">
        <div class="tech-box">
          <h4>Privacy-First Architecture</h4>
          <p>
            Predicted coding sequences are converted into MD5 hashes locally on your infrastructure.
            These cryptographic hashes cannot be reverse-engineered, ensuring your sequence information
            never leaves your systems.
          </p>
        </div>
        <div class="tech-box">
          <h4>Comprehensive Database</h4>
          <p>
            Hash queries are performed against a comprehensive database derived from UniRef protein
            clusters. Hash matches enable instant retrieval of functional classifications, database
            cross-references, and metadata.
          </p>
        </div>
        <div class="tech-box">
          <h4>Hybrid Annotation Strategy</h4>
          <p>
            For sequences without hash matches, AI-DB seamlessly transitions to classical alignment-based
            annotation using Diamond searches against curated reference databases.
          </p>
        </div>
        <div class="tech-box">
          <h4>Built on Bakta</h4>
          <p>
            AI-DB builds upon the widely-used Bakta annotation framework, functioning as both a standalone
            tool and as an acceleration layer for large-scale comparative genomics projects.
          </p>
        </div>
      </div>
    </section>

    <!-- CTA Section -->
    <section class="section cta-section">
      <h3 class="section-title">Start Using AI-DB</h3>
      <p class="section-intro">
        Accelerate your microbial genomics research while maintaining complete data sovereignty.
      </p>
      <div class="cta-buttons">
        <a href="#" class="btn btn-primary">Access AI-DB</a>
        <a href="https://github.com/hansen-maria/AI-DB-Web" class="btn btn-secondary">View on GitHub</a>
      </div>
    </section>

    <!-- Footer -->
    <footer class="footer">
      <p>&copy; 2025 AI-DB Project. Built for the genomics research community.</p>
    </footer>
  </div>
</template>

<style scoped>
.landing {
  width: 100%;
}

/* Header */
.header {
  padding: 2rem 0;
  border-bottom: 1px solid var(--color-border);
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 1rem;
}

.theme-toggle {
  flex-shrink: 0;
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
  transform: scale(1.05);
}

.theme-toggle:active {
  transform: scale(0.95);
}

.theme-toggle svg {
  display: block;
}

.logo {
  display: flex;
  align-items: center;
  gap: 1rem;
  flex-wrap: wrap;
}

.logo-image {
  flex-shrink: 0;
}

.logo-placeholder {
  width: 50px;
  height: 50px;
  aspect-ratio: 1 / 1;
  background: linear-gradient(135deg, hsla(160, 100%, 37%, 1) 0%, hsla(160, 100%, 27%, 1) 100%);
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  font-weight: 600;
  font-size: 0.7rem;
  letter-spacing: 0.5px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  transition: transform 0.3s, box-shadow 0.3s;
}

.logo-placeholder:hover {
  transform: translateY(-2px);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
}

.logo-text {
  display: flex;
  align-items: baseline;
  gap: 1rem;
  flex-wrap: wrap;
}

.logo h1 {
  font-size: 1.75rem;
  font-weight: 600;
  margin: 0;
  color: var(--color-heading);
}

.tagline {
  font-size: 0.95rem;
  color: var(--color-text);
  opacity: 0.7;
}

/* Hero Section */
.hero {
  padding: 4rem 0 3rem;
  text-align: center;
}

.hero-title {
  font-size: 2.25rem;
  font-weight: 600;
  line-height: 1.3;
  margin: 0 0 1.5rem 0;
  color: var(--color-heading);
}

.hero-description {
  font-size: 1.15rem;
  color: var(--color-text);
  opacity: 0.8;
  max-width: 700px;
  margin: 0 auto 2rem;
}

.cta-buttons {
  display: flex;
  gap: 1rem;
  justify-content: center;
  flex-wrap: wrap;
}

.btn {
  padding: 0.75rem 1.5rem;
  font-size: 0.95rem;
  font-weight: 500;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.3s;
  text-decoration: none;
  display: inline-block;
}

.btn-primary {
  background: hsla(160, 100%, 37%, 1);
  color: white;
}

.btn-primary:hover {
  background: hsla(160, 100%, 32%, 1);
}

.btn-secondary {
  background: transparent;
  color: hsla(160, 100%, 37%, 1);
  border: 1px solid hsla(160, 100%, 37%, 1);
}

.btn-secondary:hover {
  background: hsla(160, 100%, 37%, 0.1);
}

/* Sections */
.section {
  padding: 3rem 0;
}

.section-alt {
  background: var(--color-background-soft);
  margin: 0 -2rem;
  padding: 3rem 2rem;
}

.section-title {
  font-size: 1.85rem;
  font-weight: 600;
  text-align: center;
  margin: 0 0 1rem 0;
  color: var(--color-heading);
}

.section-intro {
  text-align: center;
  font-size: 1.05rem;
  color: var(--color-text);
  opacity: 0.8;
  max-width: 700px;
  margin: 0 auto 2.5rem;
}

/* Features Grid */
.features-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 2rem;
  margin-top: 2.5rem;
}

.feature h4 {
  font-size: 1.1rem;
  font-weight: 600;
  margin: 0 0 0.5rem 0;
  color: var(--color-heading);
}

.feature p {
  color: var(--color-text);
  opacity: 0.8;
  margin: 0;
  line-height: 1.6;
}

/* Workflow */
.workflow {
  max-width: 800px;
  margin: 0 auto;
}

.workflow-step {
  display: flex;
  gap: 1.25rem;
  margin-bottom: 1.75rem;
  align-items: flex-start;
}

.step-number {
  flex-shrink: 0;
  width: 36px;
  height: 36px;
  background: hsla(160, 100%, 37%, 1);
  color: white;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
  font-size: 1rem;
}

.step-content h4 {
  font-size: 1.05rem;
  font-weight: 600;
  margin: 0 0 0.35rem 0;
  color: var(--color-heading);
}

.step-content p {
  color: var(--color-text);
  opacity: 0.8;
  margin: 0;
  line-height: 1.6;
}

/* Technical Content */
.technical-content {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: 1.5rem;
  margin-top: 2rem;
}

.tech-box {
  padding: 1.5rem;
  background: var(--color-background-soft);
  border-radius: 8px;
  border: 1px solid var(--color-border);
  transition: border-color 0.3s;
}

.tech-box:hover {
  border-color: var(--color-border-hover);
}

.tech-box h4 {
  font-size: 1.05rem;
  font-weight: 600;
  margin: 0 0 0.5rem 0;
  color: var(--color-heading);
}

.tech-box p {
  color: var(--color-text);
  opacity: 0.8;
  margin: 0;
  font-size: 0.95rem;
  line-height: 1.6;
}

/* CTA Section */
.cta-section {
  text-align: center;
  padding: 4rem 0;
}

/* Footer */
.footer {
  border-top: 1px solid var(--color-border);
  padding: 2rem 0;
  text-align: center;
  color: var(--color-text);
  opacity: 0.7;
  font-size: 0.9rem;
  margin-top: 2rem;
}

.footer p {
  margin: 0;
}

/* Responsive */
@media (max-width: 768px) {
  .hero-title {
    font-size: 1.75rem;
  }

  .hero-description {
    font-size: 1rem;
  }

  .section-title {
    font-size: 1.5rem;
  }

  .features-grid {
    grid-template-columns: 1fr;
  }

  .technical-content {
    grid-template-columns: 1fr;
  }

  .logo {
    flex-direction: row;
    align-items: center;
    gap: 0.75rem;
  }

  .logo-text {
    flex-direction: column;
    align-items: flex-start;
    gap: 0.25rem;
  }
}
</style>