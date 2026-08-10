<script setup lang="ts">
import { RouterLink } from 'vue-router'
</script>

<template>
  <div class="landing">
    <!-- Hero Section -->
    <section class="hero">
      <h2 class="hero-title">Instant Protein Annotation via Hash Lookup</h2>
      <p class="hero-description">
        AI-DB is the fast first step in protein annotation: it matches your protein sequences
        against a database of already-characterized proteins using cryptographic hashes, returning
        annotations in seconds for known sequences — no alignment required.
      </p>
      <p class="hero-subnote">
        Not a match? AI-DB lets you send unmatched sequences straight on to Bakta or Psos for
        further annotation. AI-DB itself does not perform comparative or downstream analyses.
      </p>
      <div class="cta-buttons">
        <RouterLink to="/submit" class="btn btn-primary">Submit Job</RouterLink>
        <a href="/api/docs" target="_blank" class="btn btn-secondary">API Documentation</a>
      </div>
    </section>

    <!-- Scope Section -->
    <section class="section scope-section">
      <h3 class="section-title">What AI-DB Does — and Doesn't</h3>
      <p class="section-intro">
        AI-DB is designed to be the quick entry point into protein annotation, not a replacement
        for a full genome analysis pipeline.
      </p>
      <div class="scope-grid">
        <div class="scope-card scope-does">
          <h4>✓ AI-DB Does</h4>
          <ul>
            <li>Look up <strong>protein</strong> sequences against a hash database of known, characterized proteins</li>
            <li>Return annotations (COG, GO, EC, cross-references) in seconds for exact matches</li>
            <li>Offer a way to send unmatched sequences on to Bakta or Psos for further annotation</li>
            <li>Grow over time as newly annotated sequences are added to the database</li>
          </ul>
        </div>
        <div class="scope-card scope-does-not">
          <h4>✕ AI-DB Does Not</h4>
          <ul>
            <li>Annotate <strong>DNA or RNA</strong> sequences — you can upload them, but hash lookup
              only works on proteins, so no matches will be found</li>
            <li>Perform comparative genomics, whole-genome annotation, or multi-step downstream analysis</li>
            <li>Replace tools like Bakta — it's a fast pre-check that complements them</li>
          </ul>
        </div>
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
          <p>Skip computationally expensive sequence alignments. Hash-based matching delivers annotations in seconds instead of hours — for already-known protein sequences.</p>
        </div>
        <div class="feature">
          <h4>Protein-Focused</h4>
          <p>Access functional classifications, database cross-references, and metadata from extensively characterized UniRef protein sequences. AI-DB works on proteins only.</p>
        </div>
        <div class="feature">
          <h4>A Starting Point, Not the Whole Pipeline</h4>
          <p>Unmatched sequences can be forwarded to Bakta or Psos for further annotation directly from the web interface.</p>
        </div>
        <div class="feature">
          <h4>Continuously Expanding</h4>
          <p>Newly characterized sequences can be incorporated into the hash database, creating a growing knowledge base.</p>
        </div>
        <div class="feature">
          <h4>Built on Bakta</h4>
          <p>Complements the trusted Bakta framework as a lightweight first pass before deeper, alignment-based annotation.</p>
        </div>
      </div>
    </section>

    <!-- How It Works -->
    <section id="how-it-works" class="section section-alt">
      <h3 class="section-title">How It Works</h3>
      <p class="section-intro">
        AI-DB is a single, fast lookup step — with the option to send unmatched sequences onward
        for further annotation.
      </p>
      <div class="workflow">
        <div class="workflow-step">
          <div class="step-number">1</div>
          <div class="step-content">
            <h4>Upload FASTA</h4>
            <p>Upload your FASTA file or paste sequence data directly into our web interface. Note: AI-DB matches
              <strong>protein</strong> sequences — DNA/RNA input can be uploaded, but won't produce any hash matches.</p>
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
            <p>Receive instant annotations for hash matches. For sequences without a match, you can optionally
              forward them to Psos or Bakta for further annotation — this is a separate, additional step, not
              something AI-DB does automatically.</p>
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
          <span class="method post">POST</span>
          <code>/api/job/{id}/bakta/ingest</code>
          <span class="endpoint-desc">Ingest Bakta results into AI-DB Annotations DB</span>
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

.hero-subnote {
  font-size: 0.95rem;
  color: var(--color-text);
  opacity: 0.7;
  max-width: 650px;
  margin: -1rem auto 2rem;
  font-style: italic;
}

.cta-buttons {
  display: flex;
  gap: 1rem;
  justify-content: center;
  flex-wrap: wrap;
}

/* Scope Section */
.scope-section {
  background: var(--color-background-soft);
  margin: 0 -2rem;
  padding: 3rem 2rem;
}

.scope-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 1.5rem;
  max-width: 900px;
  margin: 0 auto;
}

.scope-card {
  padding: 1.5rem 1.75rem;
  border-radius: 8px;
  border: 1px solid var(--color-border);
  background: var(--color-background);
}

.scope-does {
  border-left: 3px solid hsla(160, 100%, 37%, 1);
}

.scope-does-not {
  border-left: 3px solid hsla(0, 70%, 55%, 1);
}

.scope-card h4 {
  font-size: 1.1rem;
  font-weight: 600;
  margin: 0 0 1rem 0;
  color: var(--color-heading);
}

.scope-card ul {
  margin: 0;
  padding-left: 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 0.6rem;
}

.scope-card li {
  color: var(--color-text);
  opacity: 0.85;
  line-height: 1.5;
  font-size: 0.95rem;
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
