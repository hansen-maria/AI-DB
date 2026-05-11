<script setup lang="ts">
const contact = {
  name: 'Maria Hansen',
  title: 'Research Assistant',
  institution: 'Bioinformatics & Systems Biology at Justus Liebig University, Giessen',
  email: 'maria.hansen@computational.bio.uni-giessen.de',
  github: 'https://github.com/hansen-maria',
  orcid: 'https://orcid.org/0009-0001-3265-8654',
  linkedin: 'https://www.linkedin.com/in/maria-hansen-3aab551a9',
}
const github_refs = {
  ai_db_repository: 'https://github.com/hansen-maria/AI-DB',
}
</script>

<template>
  <div class="contact-page">
    <div class="contact-header">
      <h2>Contact</h2>
      <p class="contact-subtitle">
        Questions, bug reports, or collaboration inquiries — feel free to reach out.
      </p>
    </div>

    <div class="contact-card">
      <!-- Identity -->
      <div class="contact-identity">
        <div class="contact-avatar" aria-hidden="true">
          {{ contact.name.split(' ').map(n => n[0]).join('') }}
        </div>
        <div>
          <h3 class="contact-name">{{ contact.name }}</h3>
          <p class="contact-title">{{ contact.title }}</p>
          <p class="contact-institution">{{ contact.institution }}</p>
        </div>
      </div>

      <hr class="divider" />

      <!-- Links -->
      <ul class="contact-links">
        <!-- Email -->
        <li>
          <a :href="`mailto:${contact.email}`" class="contact-link">
            <span class="contact-link__icon">
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24"
                   fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z"/>
                <polyline points="22,6 12,13 2,6"/>
              </svg>
            </span>
            <span class="contact-link__label">Email</span>
            <span class="contact-link__value">{{ contact.email }}</span>
          </a>
        </li>

        <!-- GitHub -->
        <li>
          <a :href="contact.github" target="_blank" rel="noopener noreferrer" class="contact-link">
            <span class="contact-link__icon">
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24"
                   fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22"/>
              </svg>
            </span>
            <span class="contact-link__label">GitHub</span>
            <span class="contact-link__value">{{ contact.github.replace('https://', '') }}</span>
          </a>
        </li>

        <!-- ORCID -->
        <li>
          <a :href="contact.orcid" target="_blank" rel="noopener noreferrer" class="contact-link">
            <span class="contact-link__icon orcid-icon">
              <!-- ORCID logo mark -->
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24">
                <path fill="currentColor" d="M12 0C5.372 0 0 5.372 0 12s5.372 12 12 12 12-5.372 12-12S18.628 0 12 0zm-1.84 5.4h1.36v1.56H10.16V5.4zm0 3.12h1.36V18.6H10.16V8.52zm3.72 0h3.72c3.54 0 5.04 2.46 5.04 5.04 0 2.52-1.44 5.04-5.04 5.04h-3.72V8.52zm1.36 1.32v7.44h2.28c2.52 0 3.72-1.56 3.72-3.72 0-2.16-1.2-3.72-3.72-3.72h-2.28z"/>
              </svg>
            </span>
            <span class="contact-link__label">ORCID</span>
            <span class="contact-link__value">{{ contact.orcid.replace('https://orcid.org/', '') }}</span>
          </a>
        </li>

        <!-- LinkedIn -->
        <li>
          <a :href="contact.linkedin" target="_blank" rel="noopener noreferrer" class="contact-link">
            <span class="contact-link__icon">
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24"
                   fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M16 8a6 6 0 0 1 6 6v7h-4v-7a2 2 0 0 0-2-2 2 2 0 0 0-2 2v7h-4v-7a6 6 0 0 1 6-6z"/>
                <rect x="2" y="9" width="4" height="12"/>
                <circle cx="4" cy="4" r="2"/>
              </svg>
            </span>
            <span class="contact-link__label">LinkedIn</span>
            <span class="contact-link__value">{{ contact.linkedin.replace('https://', '') }}</span>
          </a>
        </li>
      </ul>
    </div>

    <!-- Bug reports hint -->
    <div class="contact-note">
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24"
           fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="10"/>
        <line x1="12" y1="8" x2="12" y2="12"/>
        <line x1="12" y1="16" x2="12.01" y2="16"/>
      </svg>
      For bug reports and feature requests, please open an issue on
      <a :href="github_refs.ai_db_repository + '/issues'" target="_blank" rel="noopener noreferrer">GitHub</a>.
    </div>
  </div>
</template>

<style scoped>
.contact-page {
  max-width: 640px;
  margin: 3rem auto;
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.contact-header h2 {
  font-size: 1.75rem;
  font-weight: 700;
  color: var(--color-heading);
  margin-bottom: 0.5rem;
}

.contact-subtitle {
  color: var(--color-text);
  opacity: 0.75;
  font-size: 0.975rem;
}

/* Card */
.contact-card {
  border: 1px solid var(--color-border);
  border-radius: 12px;
  background: var(--color-background-soft);
  padding: 2rem;
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

/* Identity block */
.contact-identity {
  display: flex;
  align-items: center;
  gap: 1.25rem;
}

.contact-avatar {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  background: hsla(160, 100%, 37%, 0.15);
  color: hsla(160, 100%, 37%, 1);
  font-size: 1.1rem;
  font-weight: 700;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  border: 2px solid hsla(160, 100%, 37%, 0.25);
  letter-spacing: 0.5px;
}

.contact-name {
  font-size: 1.1rem;
  font-weight: 600;
  color: var(--color-heading);
  margin: 0 0 0.2rem;
}

.contact-title {
  font-size: 0.875rem;
  color: var(--color-text);
  margin: 0 0 0.15rem;
}

.contact-institution {
  font-size: 0.825rem;
  color: var(--color-text);
  opacity: 0.7;
  margin: 0;
}

.divider {
  border: none;
  border-top: 1px solid var(--color-border);
  margin: 0;
}

/* Links list */
.contact-links {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
}

.contact-link {
  display: flex;
  align-items: center;
  gap: 0.875rem;
  padding: 0.65rem 0.75rem;
  border-radius: 8px;
  text-decoration: none;
  color: var(--color-text);
  transition: background 0.18s, color 0.18s;
}

.contact-link:hover {
  background: hsla(160, 100%, 37%, 0.08);
  color: hsla(160, 100%, 37%, 1);
}

.contact-link__icon {
  width: 20px;
  height: 20px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-text);
  opacity: 0.6;
}

.contact-link:hover .contact-link__icon {
  opacity: 1;
  color: hsla(160, 100%, 37%, 1);
}

.contact-link__label {
  font-size: 0.875rem;
  font-weight: 500;
  color: var(--color-heading);
  min-width: 80px;
}

.contact-link__value {
  font-size: 0.875rem;
  color: var(--color-text);
  opacity: 0.75;
  word-break: break-all;
}

/* Note */
.contact-note {
  display: flex;
  align-items: flex-start;
  gap: 0.5rem;
  font-size: 0.875rem;
  color: var(--color-text);
  opacity: 0.75;
  padding: 0.75rem 1rem;
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: var(--color-background-mute);
  line-height: 1.5;
}

.contact-note svg {
  flex-shrink: 0;
  margin-top: 2px;
}

.contact-note a {
  color: hsla(160, 100%, 37%, 1);
  text-decoration: none;
  opacity: 1;
}

.contact-note a:hover {
  text-decoration: underline;
}

@media (max-width: 640px) {
  .contact-page {
    margin: 1.5rem auto;
  }

  .contact-card {
    padding: 1.25rem;
  }

  .contact-identity {
    flex-direction: column;
    align-items: flex-start;
    gap: 0.75rem;
  }

  .contact-link__label {
    min-width: 60px;
  }
}
</style>