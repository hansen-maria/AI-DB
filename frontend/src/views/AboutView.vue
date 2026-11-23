<script setup lang="ts">
import { ref } from 'vue'
import { RouterLink } from 'vue-router'
import { SUPPORT_EMAIL, REPO_URL, ISSUES_URL } from '@/constants/contact'

const sections = [
  { id: 'help', label: 'Help' },
  { id: 'faq', label: 'FAQ' },
  { id: 'manual', label: 'Manual' },
  { id: 'cite', label: 'How to Cite' },
]

interface FaqItem {
  q: string
  a: string
}

// Answers may contain simple inline HTML (<strong>, <code>, <a>); content is static.
const faq: FaqItem[] = [
  {
    q: 'What does AI-DB do?',
    a: 'AI-DB is the fast first step in protein annotation. Each submitted protein sequence is converted into an MD5 hash and looked up in databases of already-annotated proteins. Exact matches return their annotation (gene, product, COG, EC, GO, database cross-references) within seconds, without any alignment.',
  },
  {
    q: 'Which input does AI-DB accept?',
    a: 'Protein sequences in FASTA format (<code>.fasta</code>, <code>.fa</code>, <code>.faa</code>, <code>.fna</code>, <code>.txt</code>, optionally gzip-compressed), up to 100 MB per file. You can also paste sequences directly. DNA or RNA can be uploaded, but the hash lookup only works on proteins, so nucleotide sequences will not produce matches.',
  },
  {
    q: 'Why did my sequence not match?',
    a: 'A hash lookup only finds <strong>identical</strong> sequences. A single amino acid difference, a truncated or extended sequence, or a trailing stop character (<code>*</code>) produces a different hash. Letter case does not matter. Unmatched sequences are not an error: you can forward them to Bakta or Psos from the job page.',
  },
  {
    q: 'Which databases are searched?',
    a: 'Sequences are looked up in the <strong>Bakta DB</strong> first, then in the AI-DB databases: <strong>AI-DB · BakRep</strong> (BakRep proteins not contained in the Bakta DB) and <strong>AI-DB · Community</strong> (annotations contributed by Bakta runs of AI-DB users). The source of each match is shown as a badge in the Sequences tab.',
  },
  {
    q: 'What does the number on the source badge mean?',
    a: 'Hovering the source badge shows in how many jobs AI-DB has already seen this sequence. Each job counts at most once.',
  },
  {
    q: 'Is my data private?',
    a: 'Your jobs are only visible to people who know the Job-ID, and only you can change, download or delete them. Sequences are sent to external services (Bakta, Psos) only when you start such an analysis yourself. Contributions to the AI-DB · Community database contain only MD5 hashes and annotation fields, never the sequences. Details are given in the <a href="/imprint#privacy">Privacy Policy</a>.',
  },
  {
    q: 'How long are my jobs kept?',
    a: 'Jobs and their sequences are deleted automatically 30 days after creation. You can delete a job earlier at any time from the job page or the job list.',
  },
  {
    q: 'Can I share results with colleagues?',
    a: 'Yes. Send the job link (it contains the Job-ID). Anyone with the link can view the results read-only; downloads, renaming, deletion and further analyses remain reserved for you as the owner.',
  },
  {
    q: 'I cleared my cookies / use a different browser. Where are my jobs?',
    a: 'In the browser, your jobs are linked to a cookie. Save your access token beforehand under <strong>Jobs → Access token</strong>; with it you can keep accessing your jobs via the REST API and the <code>aidb</code> command-line client. Without the token, existing jobs can only be opened read-only via their link.',
  },
  {
    q: 'Can I use AI-DB in scripts or pipelines?',
    a: 'Yes. AI-DB has a documented REST API (<a href="/api/docs" target="_blank" rel="noopener">Swagger UI</a>) that authenticates with an access token instead of cookies, and a command-line client <code>aidb</code>. See the Manual below.',
  },
]

const openFaq = ref<Set<number>>(new Set())
const toggleFaq = (i: number) => {
  const next = new Set(openFaq.value)
  if (next.has(i)) next.delete(i)
  else next.add(i)
  openFaq.value = next
}

// ── Citation ──
// TODO: replace with the journal reference and DOI once the AI-DB paper is published.
const CITATION_TEXT =
  'Hansen M, Blom J, Schwengers O, Förster F, Goesmann A. AI-DB: A hash-based, alignment-free web service for rapid bacterial protein identification\n. Manuscript in preparation. https://ai-db.computational.bio'

const CITATION_BIBTEX = `@misc{hansen_aidb,
  author       = {Hansen, Maria and Blom, Jochen and Schwengers, Oliver and F{\\"o}rster, Frank and Goesmann, Alexander},
  title        = {{AI-DB}: A hash-based, alignment-free web service for rapid bacterial protein identification},
  howpublished = {\\url{https://ai-db.computational.bio}},
  note         = {Manuscript in preparation. Source code: ${REPO_URL}},
  year         = {2026}
}`

const relatedCitations = [
  {
    name: 'Bakta',
    when: 'if you use annotations from the Bakta DB or run Bakta from AI-DB',
    text: 'Schwengers O, Jelonek L, Dieckmann MA, Beyvers S, Blom J, Goesmann A. Bakta: rapid and standardized annotation of bacterial genomes via alignment-free sequence identification. Microbial Genomics 2021;7(11):000685.',
    doi: '10.1099/mgen.0.000685',
  },
  {
    name: 'BakRep',
    when: 'if your results contain AI-DB · BakRep matches',
    text: 'Fenske L, Jelonek L, Goesmann A, Schwengers O. BakRep – a searchable large-scale web repository for bacterial genomes, characterizations and metadata. Microbial Genomics 2024;10:001305.',
    doi: '10.1099/mgen.0.001305',
  },
]

const copiedKey = ref<string | null>(null)
const copy = async (key: string, text: string) => {
  try {
    await navigator.clipboard.writeText(text)
    copiedKey.value = key
    setTimeout(() => {
      if (copiedKey.value === key) copiedKey.value = null
    }, 2000)
  } catch {
    copiedKey.value = null
  }
}
</script>

<template>
  <div class="about-page">
    <div class="page-header">
      <h2>About AI-DB</h2>
      <p>Help, frequently asked questions, the user manual and how to cite AI-DB.</p>
    </div>

    <!-- Section navigation -->
    <nav class="section-nav" aria-label="About sections">
      <RouterLink v-for="s in sections" :key="s.id" :to="{ hash: `#${s.id}` }" class="section-nav__link">
        {{ s.label }}
      </RouterLink>
    </nav>

    <!-- ── Help ── -->
    <section id="help" class="about-section">
      <h3 class="about-section__title">Help</h3>
      <p class="about-section__intro">
        Most questions are answered in the <RouterLink :to="{ hash: '#faq' }">FAQ</RouterLink> and the
        <RouterLink :to="{ hash: '#manual' }">Manual</RouterLink> below. If you still need help, these are the
        best ways to reach us:
      </p>
      <div class="help-grid">
        <a :href="`mailto:${SUPPORT_EMAIL}`" class="help-card">
          <span class="help-card__title">Email support</span>
          <span class="help-card__text">Questions about results, data deletion requests or collaboration inquiries.</span>
          <span class="help-card__value">{{ SUPPORT_EMAIL }}</span>
        </a>
        <a :href="ISSUES_URL" target="_blank" rel="noopener noreferrer" class="help-card">
          <span class="help-card__title">Bugs &amp; feature requests</span>
          <span class="help-card__text">Report problems or suggest improvements as an issue on GitHub.</span>
          <span class="help-card__value">GitHub Issues</span>
        </a>
        <a href="/api/docs" target="_blank" rel="noopener" class="help-card">
          <span class="help-card__title">API documentation</span>
          <span class="help-card__text">All REST endpoints with request and response schemas (Swagger UI).</span>
          <span class="help-card__value">/api/docs</span>
        </a>
      </div>
      <p class="about-note">
        When writing to us about a specific job, please include its Job-ID (shown on the job page and in its link).
      </p>
    </section>

    <!-- ── FAQ ── -->
    <section id="faq" class="about-section">
      <h3 class="about-section__title">Frequently Asked Questions</h3>
      <ul class="faq-list">
        <li v-for="(item, i) in faq" :key="item.q" class="faq-item" :class="{ open: openFaq.has(i) }">
          <button
            class="faq-item__question"
            :aria-expanded="openFaq.has(i)"
            :aria-controls="`faq-answer-${i}`"
            @click="toggleFaq(i)"
          >
            <span>{{ item.q }}</span>
            <svg class="faq-item__chevron" xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24"
                 fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <polyline points="6 9 12 15 18 9"/>
            </svg>
          </button>
          <!-- eslint-disable-next-line vue/no-v-html -- static, trusted content -->
          <div v-show="openFaq.has(i)" :id="`faq-answer-${i}`" class="faq-item__answer" v-html="item.a"></div>
        </li>
      </ul>
    </section>

    <!-- ── Manual ── -->
    <section id="manual" class="about-section">
      <h3 class="about-section__title">Manual</h3>

      <div class="manual-step">
        <div class="manual-step__number">1</div>
        <div class="manual-step__body">
          <h4>Submit a job</h4>
          <p>
            Open <RouterLink to="/submit">Submit Job</RouterLink>, upload a protein FASTA file (drag &amp; drop, max.
            100 MB, gzip supported) or paste sequences, and optionally give the job a name. After submission you are
            redirected to the job page, which updates automatically while the job is processed.
          </p>
        </div>
      </div>

      <div class="manual-step">
        <div class="manual-step__number">2</div>
        <div class="manual-step__body">
          <h4>Read the overview</h4>
          <p>
            The <strong>Overview</strong> tab summarizes the job: number of sequences, hash matches per source and
            the share of annotated sequences. From here you can jump to the sequences, the functional analysis, or
            annotate the unmatched sequences with Bakta.
          </p>
        </div>
      </div>

      <div class="manual-step">
        <div class="manual-step__number">3</div>
        <div class="manual-step__body">
          <h4>Explore the sequences</h4>
          <p>
            The <strong>Sequences</strong> tab lists every sequence with gene, product, length and links to
            UniRef, UniParc and NCBI. Search by ID, gene or product, use the quick filters (All, Matched, per
            source, No Match) or open the advanced filters for length, COG category, EC class, gene name and
            function description. Filtered subsets can be downloaded directly.
          </p>
        </div>
      </div>

      <div class="manual-step">
        <div class="manual-step__number">4</div>
        <div class="manual-step__body">
          <h4>Analyse the functions</h4>
          <p>
            <strong>Functional Analysis</strong> shows the annotation rate and the most frequent genes and products,
            COG categories, EC classes and GO terms. <strong>Visualization</strong> presents the functional
            classification as interactive hierarchical charts.
          </p>
        </div>
      </div>

      <div class="manual-step">
        <div class="manual-step__number">5</div>
        <div class="manual-step__body">
          <h4>Annotate unmatched sequences</h4>
          <p>
            Sequences without a hash match can be forwarded to <strong>Bakta</strong> (full protein annotation) or
            <strong>Psos</strong> (e.g. signal peptides and transmembrane domains). Nothing is sent before you click
            the corresponding button. Bakta results are added to the AI-DB · Community database by default, so
            future jobs recognize these sequences; you can switch this off before starting the run. Only hashes
            and annotation fields are stored.
          </p>
        </div>
      </div>

      <div class="manual-step">
        <div class="manual-step__number">6</div>
        <div class="manual-step__body">
          <h4>Download results</h4>
          <p>
            Download the full results as <strong>TSV</strong> (spreadsheets, R, Python), <strong>JSON</strong>
            (programmatic access), <strong>FASTA</strong> (downstream tools) or <strong>GFF3</strong> (genome
            browsers). Bakta and Psos results have their own download buttons in the respective panels.
          </p>
        </div>
      </div>

      <div class="manual-step">
        <div class="manual-step__number">7</div>
        <div class="manual-step__body">
          <h4>Manage your jobs</h4>
          <p>
            <RouterLink to="/jobs">Jobs</RouterLink> lists all jobs of your browser with search and status filters.
            Jobs can be renamed, deleted individually or in bulk, and failed jobs can be retried. Jobs are kept for
            30 days. Under <strong>Jobs → Access token</strong> you find the token that identifies you as the owner.
          </p>
        </div>
      </div>

      <div class="manual-step">
        <div class="manual-step__number">8</div>
        <div class="manual-step__body">
          <h4>Use the API or the command line</h4>
          <p>
            Send your access token as <code>Authorization: Bearer &lt;token&gt;</code> to use the
            <a href="/api/docs" target="_blank" rel="noopener">REST API</a>, or use the command-line client:
          </p>
          <pre class="code-block"><code>aidb auth import
aidb annotate proteins.faa -f tsv,gff3 -d results/</code></pre>
          <p>
            Installation and all commands are described in the
            <a :href="`${REPO_URL}/tree/main/crates/aidb-cli`" target="_blank" rel="noopener noreferrer">CLI README</a>.
          </p>
        </div>
      </div>
    </section>

    <!-- ── How to Cite ── -->
    <section id="cite" class="about-section">
      <h3 class="about-section__title">How to Cite</h3>
      <p class="about-section__intro">
        If AI-DB contributed to your work, please cite it as follows. The publication is in preparation; this
        reference will be updated once it is available.
      </p>

      <div class="cite-box">
        <p class="cite-box__text">{{ CITATION_TEXT }}</p>
        <button class="copy-btn" :class="{ copied: copiedKey === 'text' }" @click="copy('text', CITATION_TEXT)">
          {{ copiedKey === 'text' ? 'Copied' : 'Copy' }}
        </button>
      </div>

      <div class="cite-box cite-box--code">
        <pre class="code-block"><code>{{ CITATION_BIBTEX }}</code></pre>
        <button class="copy-btn" :class="{ copied: copiedKey === 'bibtex' }" @click="copy('bibtex', CITATION_BIBTEX)">
          {{ copiedKey === 'bibtex' ? 'Copied' : 'Copy BibTeX' }}
        </button>
      </div>

      <h4 class="cite-subtitle">Please also cite</h4>
      <ul class="related-list">
        <li v-for="c in relatedCitations" :key="c.name">
          <strong>{{ c.name }}</strong> <span class="related-when">— {{ c.when }}</span><br>
          {{ c.text }}
          <a :href="`https://doi.org/${c.doi}`" target="_blank" rel="noopener noreferrer">doi:{{ c.doi }}</a>
        </li>
        <li>
          <strong>Psos</strong> <span class="related-when">— if you use Psos results</span><br>
          Please follow the citation notes on the
          <a href="https://psos.computational.bio" target="_blank" rel="noopener noreferrer">Psos website</a>.
        </li>
      </ul>
    </section>
  </div>
</template>

<style scoped>
.about-page {
  max-width: 860px;
  margin: 2rem auto 3rem;
}

.page-header {
  margin-bottom: 1.5rem;
}

.page-header h2 {
  font-size: 1.75rem;
  font-weight: 700;
  color: var(--color-heading);
  margin-bottom: 0.5rem;
}

.page-header p {
  color: var(--color-text);
  opacity: 0.75;
}

/* Section navigation */
.section-nav {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  padding-bottom: 1rem;
  margin-bottom: 1rem;
  border-bottom: 1px solid var(--color-border);
}

.section-nav__link {
  padding: 0.4rem 0.9rem;
  border: 1px solid var(--color-border);
  border-radius: 999px;
  font-size: 0.9rem;
  text-decoration: none;
  color: var(--color-text);
  transition: all 0.18s;
}

.section-nav__link:hover {
  border-color: hsla(160, 100%, 37%, 0.5);
  color: hsla(160, 100%, 37%, 1);
  background: hsla(160, 100%, 37%, 0.08);
}

/* Sections */
.about-section {
  padding: 2rem 0;
  border-bottom: 1px solid var(--color-border);
}

.about-section:last-of-type {
  border-bottom: none;
}

.about-section__title {
  font-size: 1.35rem;
  font-weight: 600;
  color: var(--color-heading);
  margin: 0 0 1rem;
}

.about-section__intro {
  margin: 0 0 1.25rem;
  line-height: 1.65;
}

.about-section a,
.faq-item__answer :deep(a) {
  color: hsla(160, 100%, 37%, 1);
  text-decoration: none;
}

.about-section a:hover,
.faq-item__answer :deep(a):hover {
  text-decoration: underline;
}

code,
.faq-item__answer :deep(code) {
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.85em;
  padding: 0.1rem 0.35rem;
  border-radius: 4px;
  background: var(--color-background-mute);
}

/* Help */
.help-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 1rem;
}

.about-section .help-card {
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
  padding: 1.1rem 1.25rem;
  border: 1px solid var(--color-border);
  border-radius: 10px;
  background: var(--color-background-soft);
  color: var(--color-text);
  text-decoration: none;
  transition: border-color 0.18s, background 0.18s;
}

.about-section .help-card:hover {
  border-color: hsla(160, 100%, 37%, 0.5);
  background: hsla(160, 100%, 37%, 0.06);
  text-decoration: none;
}

.help-card__title {
  font-weight: 600;
  color: var(--color-heading);
}

.help-card__text {
  font-size: 0.875rem;
  opacity: 0.8;
  line-height: 1.5;
  flex: 1;
}

.help-card__value {
  font-size: 0.85rem;
  color: hsla(160, 100%, 37%, 1);
  word-break: break-all;
}

.about-note {
  margin: 1rem 0 0;
  font-size: 0.875rem;
  opacity: 0.75;
}

/* FAQ */
.faq-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.faq-item {
  border: 1px solid var(--color-border);
  border-radius: 8px;
  background: var(--color-background-soft);
}

.faq-item.open {
  border-color: hsla(160, 100%, 37%, 0.4);
}

.faq-item__question {
  width: 100%;
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 1rem;
  padding: 0.85rem 1rem;
  background: none;
  border: none;
  cursor: pointer;
  text-align: left;
  font: inherit;
  font-weight: 500;
  color: var(--color-heading);
}

.faq-item__chevron {
  flex-shrink: 0;
  transition: transform 0.2s;
  opacity: 0.6;
}

.faq-item.open .faq-item__chevron {
  transform: rotate(180deg);
}

.faq-item__answer {
  padding: 0 1rem 1rem;
  line-height: 1.65;
  font-size: 0.95rem;
}

/* Manual */
.manual-step {
  display: flex;
  gap: 1rem;
  margin-bottom: 1.5rem;
}

.manual-step:last-child {
  margin-bottom: 0;
}

.manual-step__number {
  width: 32px;
  height: 32px;
  flex-shrink: 0;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 600;
  font-size: 0.9rem;
  color: hsla(160, 100%, 37%, 1);
  background: hsla(160, 100%, 37%, 0.12);
  border: 1px solid hsla(160, 100%, 37%, 0.3);
}

.manual-step__body {
  min-width: 0;
}

.manual-step__body h4 {
  margin: 0.25rem 0 0.4rem;
  font-size: 1.05rem;
  font-weight: 600;
  color: var(--color-heading);
}

.manual-step__body p {
  margin: 0 0 0.5rem;
  line-height: 1.65;
}

.code-block {
  margin: 0.5rem 0 0.75rem;
  padding: 0.85rem 1rem;
  border-radius: 8px;
  border: 1px solid var(--color-border);
  background: var(--color-background-mute);
  overflow-x: auto;
  font-size: 0.85rem;
  line-height: 1.55;
}

.code-block code {
  padding: 0;
  background: none;
  font-size: inherit;
}

/* Citation */
.cite-box {
  display: flex;
  align-items: flex-start;
  gap: 1rem;
  padding: 1rem 1.25rem;
  margin-bottom: 1rem;
  border: 1px solid var(--color-border);
  border-left: 3px solid hsla(160, 100%, 37%, 1);
  border-radius: 8px;
  background: var(--color-background-soft);
}

.cite-box--code {
  padding: 0.25rem 1rem 0.25rem 0.25rem;
  align-items: center;
}

.cite-box--code .code-block {
  flex: 1;
  min-width: 0;
  border: none;
  background: none;
  margin: 0;
}

.cite-box__text {
  flex: 1;
  margin: 0;
  line-height: 1.6;
}

.copy-btn {
  flex-shrink: 0;
  padding: 0.4rem 0.8rem;
  border-radius: 6px;
  border: 1px solid var(--color-border);
  background: var(--color-background);
  color: var(--color-text);
  font-size: 0.8rem;
  cursor: pointer;
  white-space: nowrap;
  transition: all 0.18s;
}

.copy-btn:hover {
  border-color: hsla(160, 100%, 37%, 0.5);
  color: hsla(160, 100%, 37%, 1);
}

.copy-btn.copied {
  border-color: hsla(160, 100%, 37%, 1);
  color: hsla(160, 100%, 37%, 1);
  background: hsla(160, 100%, 37%, 0.08);
}

.cite-subtitle {
  margin: 1.5rem 0 0.75rem;
  font-size: 1.05rem;
  font-weight: 600;
  color: var(--color-heading);
}

.related-list {
  margin: 0;
  padding-left: 1.25rem;
  display: flex;
  flex-direction: column;
  gap: 0.85rem;
  line-height: 1.6;
}

.related-when {
  opacity: 0.7;
  font-size: 0.9rem;
}

@media (max-width: 640px) {
  .about-page {
    margin: 1rem auto 2rem;
  }

  .cite-box {
    flex-direction: column;
  }

  .cite-box--code {
    align-items: stretch;
    padding: 0.25rem 0.75rem 0.75rem;
  }
}
</style>
