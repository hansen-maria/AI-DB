<script setup lang="ts">
const REPO_URL = 'https://github.com/hansen-maria/AI-DB'

const LAST_EDITED = '2026-10-19'
const lastEditedLabel = `Last edited: ${new Date(LAST_EDITED).toLocaleDateString('en-GB', {
  day: '2-digit',
  month: 'long',
  year: 'numeric',
})}`
</script>

<template>
  <div class="legal-page">
    <div class="page-header">
      <h2>Terms of Service</h2>
      <p>Please read these terms carefully before using AI-DB.</p>
    </div>

    <div class="legal-body">

      <section class="legal-section">
        <p>
          These Terms of Service govern the use of the AI-DB web service, hereafter
          referred to as <strong>AI-DB</strong>, for hash-based, alignment-free annotation
          of bacterial protein sequences. The operator, hereafter referred to as the
          <strong>Operator</strong>, is the Chair of Bioinformatics and Systems Biology at
          Justus Liebig University Giessen (JLU). The terms of service are binding for all
          users, regardless of how access is provided.
        </p>
      </section>

      <section class="legal-section">
        <h3>Consent to Use</h3>
        <p>
          By using AI-DB, you agree to comply with these Terms of Service and accept them
          as a legal agreement between you and the Operator.
        </p>
      </section>

      <section class="legal-section">
        <h3>Authorization</h3>
        <p>
          AI-DB can be used without registration. You agree and warrant that you are
          authorized to upload and process the sequences you submit through this web
          service. You are solely responsible for the data you upload, including compliance
          with applicable law and any agreements with third parties governing the use of
          that data.
        </p>
      </section>

      <section class="legal-section">
        <h3>Acceptable Use</h3>
        <p>
          You agree to use AI-DB exclusively for lawful, research-oriented purposes.
          In particular, you must not:
        </p>
        <ul>
          <li>submit data other than protein sequences intended for annotation, in particular data containing personally identifiable information (PII) or human genetic data;</li>
          <li>attempt to circumvent rate limits, upload limits, or access controls;</li>
          <li>use the service in a manner that disrupts or degrades performance for other users, for example through automated mass submissions;</li>
          <li>deliberately submit sequences or trigger annotations intended to corrupt or manipulate the AI-DB annotations DB;</li>
          <li>submit data whose processing would violate applicable law or third-party rights.</li>
        </ul>
      </section>

      <section class="legal-section">
        <h3>Availability</h3>
        <p>
          The Operator's responsibilities include planning, providing, operating,
          maintaining, and servicing AI-DB. These services are provided within the scope
          of available resources, in particular personnel, compute, and storage capacity.
          No guarantees can be given with regard to availability or freedom from errors.
          In particular, no service level agreement is entered into.
        </p>
      </section>

      <section class="legal-section">
        <h3>External Annotation Services</h3>
        <p>
          Sequences without a match in the Bakta database or the AI-DB annotations DB can
          optionally be forwarded to external annotation services (Bakta Web API, including
          Baktfold, and Psos). Forwarding is triggered only by you. The use of these services
          is additionally subject to their respective terms of service. The Operator has no
          influence on their availability, processing times, or results.
        </p>
      </section>

      <section class="legal-section">
        <h3>Scientific Results</h3>
        <p>
          AI-DB is intended as a fast first step in an annotation pipeline. Annotations are
          assigned on the basis of exact sequence identity and may be incomplete, outdated,
          or incorrect, in particular annotations contributed by the community. Results are
          provided for research purposes only and should be verified before being used as
          the basis for further scientific conclusions or publications.
        </p>
      </section>

      <section class="legal-section">
        <h3>Data Protection and Confidentiality</h3>
        <p>
          We are committed to treating the sequences and analysis results you submit
          confidentially. Jobs are identified by a random job ID; anyone who knows a job ID
          can access its results. Submitted sequences and jobs are deleted automatically
          30 days after creation. We will not share your data with third parties without
          your instruction, unless required by law or necessary to provide the web service.
        </p>
        <p>
          For full details on how personal data is processed, please refer to the
          <RouterLink :to="{ path: '/imprint', hash: '#privacy' }">Privacy Policy</RouterLink>.
        </p>
      </section>

      <section class="legal-section">
        <h3>Contribution to the AI-DB Annotations DB</h3>
        <p>
          By default, annotations obtained for previously unmatched sequences are added to
          the AI-DB annotations DB and made available to all users for future lookups. Only
          the MD5 hash of each sequence and the derived annotation are stored — not the
          sequence itself and no reference to your job. You can disable this contribution at
          any time in the job view before starting an external analysis.
        </p>
        <p>
          By leaving the contribution enabled, you agree that the contributed annotations may
          be stored permanently, used, and made available as part of the AI-DB annotations DB
          without restriction. Contributed entries cannot be assigned to individual users and
          therefore cannot be withdrawn retrospectively.
        </p>
      </section>

      <section class="legal-section">
        <h3>Security</h3>
        <p>
          We implement appropriate technical and organisational measures to ensure the
          security of submitted data and to prevent unauthorised access, disclosure, or loss
          of data.
        </p>
      </section>

      <section class="legal-section">
        <h3>Intellectual Property</h3>
        <p>
          You retain all intellectual property rights in the sequences you submit. The
          annotation data provided through AI-DB originates in part from third-party databases
          (in particular Bakta, UniRef, UniParc, and NCBI) and is subject to their respective
          licence terms.
        </p>
      </section>

      <section class="legal-section">
        <h3>Disclaimer of Liability</h3>
        <p>
          We accept no liability for losses or damages arising from the use of AI-DB,
          including the use of annotation results, unless such losses result from gross
          negligence or wilful misconduct on the part of the Operator.
        </p>
      </section>

      <section class="legal-section">
        <h3>Data Deletion</h3>
        <p>
          You have the right to request early deletion of a job and its sequences. Please
          send your request, including the job ID, to the contact address given in the
          <RouterLink to="/imprint">Imprint</RouterLink>. We will delete the data within a
          reasonable period, unless legal or legitimate reasons for retention apply. For
          further rights under the GDPR, see the
          <RouterLink :to="{ path: '/imprint', hash: '#privacy' }">Privacy Policy</RouterLink>.
        </p>
      </section>

      <section class="legal-section">
        <h3>Software License</h3>
        <p>
          The AI-DB source code is dual-licensed under the <strong>Apache License 2.0</strong>
          and the <strong>MIT License</strong>. You may choose either licence and are free to
          use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the
          software, subject to the conditions of the licence you choose.
        </p>
        <p>
          These licences apply to the source code only and do not grant any rights to use the
          AI-DB name, logo, or branding, nor do they imply endorsement of derivative works by
          Justus Liebig University Giessen, NFDI4Biodiversity, or the German Research
          Foundation (DFG).
        </p>
        <p>
          The source code and full licence texts are available in the
          <a :href="REPO_URL" target="_blank" rel="noopener">AI-DB GitHub repository</a>.
        </p>
      </section>

      <section class="legal-section">
        <h3>Changes to These Terms</h3>
        <p>
          We reserve the right to update these Terms of Service at any time. Changes will be
          notified by publishing the revised version on this page. Your continued use of AI-DB
          after such changes constitutes acceptance of the updated terms.
        </p>
      </section>

      <p class="last-updated">
        <time :datetime="LAST_EDITED">{{ lastEditedLabel }}</time>
      </p>

    </div>
  </div>
</template>

<style scoped>
.legal-page {
  --legal-primary: hsla(160, 100%, 37%, 1);
  width: 100%;
  max-width: 760px;
  margin: 0 auto;
}

.page-header {
  padding: 2.5rem 0 1.5rem;
  border-bottom: 1px solid var(--color-border);
  margin-bottom: 2.5rem;
}

.page-header h2 {
  font-size: 2rem;
  font-weight: 600;
  color: var(--color-heading);
  margin: 0 0 0.5rem 0;
}

.page-header p {
  color: var(--color-text);
  opacity: 0.8;
  font-size: 1.05rem;
  margin: 0;
}

.legal-section {
  padding-bottom: 2rem;
  margin-bottom: 2rem;
  border-bottom: 1px solid var(--color-border);
}

.legal-section:last-of-type {
  border-bottom: none;
  margin-bottom: 0;
  padding-bottom: 0;
}

.legal-section h3 {
  font-size: 1.1rem;
  font-weight: 600;
  color: var(--color-heading);
  margin: 0 0 0.75rem 0;
}

.legal-section p {
  color: var(--color-text);
  opacity: 0.85;
  font-size: 0.95rem;
  line-height: 1.7;
  margin: 0 0 0.75rem 0;
}

.legal-section p:last-child {
  margin-bottom: 0;
}

.legal-section ul {
  color: var(--color-text);
  opacity: 0.85;
  font-size: 0.95rem;
  line-height: 1.7;
  margin: 0 0 0.75rem 1.25rem;
  padding: 0;
}

.legal-section ul:last-child {
  margin-bottom: 0;
}

.legal-section li {
  margin-bottom: 0.35rem;
}

.legal-section a {
  color: var(--legal-primary);
  text-decoration: none;
}

.legal-section a:hover {
  text-decoration: underline;
}

.last-updated {
  font-size: 0.82rem;
  color: var(--color-text);
  opacity: 0.5;
  margin-top: 2.5rem;
  padding-top: 1.5rem;
  border-top: 1px solid var(--color-border);
}

@media (max-width: 768px) {
  .page-header {
    padding-top: 1.5rem;
  }

  .page-header h2 {
    font-size: 1.6rem;
  }
}
</style>