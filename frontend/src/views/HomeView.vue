<script setup lang="ts">
import { RouterLink } from 'vue-router'
</script>

<template>
  <div class="landing">
    <!-- Hero Section -->
    <section class="hero">
      <h2 class="hero-title">Hash-Based Annotation Service for Microbial Sequencing Data</h2>
      <p class="hero-description">
        AI-DB accelerates the analysis of microbial sequencing data while ensuring data privacy
        and sovereignty through cryptographic hash-based annotations.
      </p>
      <div class="cta-buttons">
        <RouterLink to="/submit" class="btn btn-primary">Submit Job</RouterLink>
        <a href="/api/docs" target="_blank" class="btn btn-secondary">API Documentation</a>
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
            <h4>Upload FASTA</h4>
            <p>Upload your FASTA file or paste sequence data directly into our web interface.</p>
          </div>
        </div>
        <div class="workflow-step">
          <div class="step-number">2</div>
          <div class="step-content">
            <h4>Hash Conversion</h4>
            <p>Sequences are converted into MD5 hashes, ensuring your original data remains private.</p>
          </div>
        </div>
        <div class="workflow-step">
          <div class="step-number">3</div>
          <div class="step-content">
            <h4>Database Query</h4>
            <p>Hashes are queried against the comprehensive hash database derived from UniRef clusters.</p>
          </div>
        </div>
        <div class="workflow-step">
          <div class="step-number">4</div>
          <div class="step-content">
            <h4>Get Results</h4>
            <p>Receive instant annotations for hash matches, with alignment fallback for novel sequences.</p>
          </div>
        </div>
      </div>
    </section>

    <!-- REST API Section -->
    <section class="section">
      <h3 class="section-title">REST API</h3>
      <p class="section-intro">
        Full programmatic access to AI-DB through our documented REST API with OpenAPI/Swagger support.
      </p>
      <div class="api-endpoints">
        <div class="endpoint">
          <span class="method post">POST</span>
          <code>/api/job/</code>
          <span class="endpoint-desc">Submit a new annotation job with FASTA data</span>
        </div>
        <div class="endpoint">
          <span class="method get">GET</span>
          <code>/api/job/{id}</code>
          <span class="endpoint-desc">Retrieve job status and results</span>
        </div>
        <div class="endpoint">
          <span class="method get">GET</span>
          <code>/api/jobs/</code>
          <span class="endpoint-desc">List all annotation jobs</span>
        </div>
        <div class="endpoint">
          <span class="method get">GET</span>
          <code>/api/job/{id}/download/{format}</code>
          <span class="endpoint-desc">Download results in different file formats</span>
        </div>
        <div class="endpoint">
          <span class="method delete">DELETE</span>
          <code>/api/job/{id}</code>
          <span class="endpoint-desc">Delete a job and its results</span>
        </div>
      </div>
      <div class="api-cta">
        <a href="/api/docs" target="_blank" class="btn btn-secondary">
          View Full API Documentation (Swagger)
        </a>
      </div>
    </section>

    <!-- CTA Section -->
    <section class="section cta-section">
      <h3 class="section-title">Start Using AI-DB</h3>
      <p class="section-intro">
        Accelerate your microbial genomics research while maintaining complete data sovereignty.
      </p>
      <div class="cta-buttons">
        <RouterLink to="/submit" class="btn btn-primary">Submit Your First Job</RouterLink>
        <a href="https://github.com/hansen-maria/AI-DB-Web" class="btn btn-secondary">View on GitHub</a>
      </div>
    </section>
  </div>
</template>

<style scoped>
.landing {
  width: 100%;
}

/* Hero Section */
.hero {
  padding: 3rem 0 2rem;
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

.feature {
  text-align: center;
  padding: 1.5rem;
  border-radius: 8px;
  background: var(--color-background-soft);
  border: 1px solid var(--color-border);
  transition: all 0.3s;
}

.feature:hover {
  border-color: var(--color-border-hover);
  transform: translateY(-2px);
}

.feature-icon {
  font-size: 2rem;
  margin-bottom: 1rem;
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
  font-size: 0.95rem;
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

/* API Endpoints */
.api-endpoints {
  max-width: 700px;
  margin: 0 auto 2rem;
}

.endpoint {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 0.75rem 1rem;
  background: var(--color-background-soft);
  border: 1px solid var(--color-border);
  border-radius: 6px;
  margin-bottom: 0.75rem;
  flex-wrap: wrap;
}

.method {
  padding: 0.25rem 0.5rem;
  border-radius: 4px;
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  min-width: 60px;
  text-align: center;
}

.method.get {
  background: #e3f2fd;
  color: #1565c0;
}

.method.post {
  background: #e8f5e9;
  color: #2e7d32;
}

.method.delete {
  background: #ffebee;
  color: #c62828;
}

.endpoint code {
  font-family: 'Monaco', 'Consolas', monospace;
  font-size: 0.9rem;
  color: var(--color-heading);
}

.endpoint-desc {
  color: var(--color-text);
  opacity: 0.8;
  font-size: 0.9rem;
  margin-left: auto;
}

.api-cta {
  text-align: center;
}

/* CTA Section */
.cta-section {
  text-align: center;
  padding: 4rem 0;
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

  .endpoint {
    flex-direction: column;
    align-items: flex-start;
    gap: 0.5rem;
  }

  .endpoint-desc {
    margin-left: 0;
  }
}
</style>
