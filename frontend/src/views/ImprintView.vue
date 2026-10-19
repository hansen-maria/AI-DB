<script setup lang="ts">
const CONTACT_EMAIL = 'maria.hansen@computational.bio.uni-giessen.de'

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
      <h2>Imprint &amp; Privacy Policy</h2>
      <p>Legal notice (§ 5 DDG) and data protection information (GDPR / DSGVO).</p>
    </div>

    <div class="legal-body">

      <!-- ── Imprint ── -->
      <section class="legal-section">
        <h3>Responsible Institution</h3>
        <address>
          President of the<br>
          <strong>Justus Liebig University Giessen (JLU)</strong><br>
          Ludwigstraße 23<br>
          35390 Gießen<br>
          Germany
        </address>
        <p>
          Justus Liebig University Giessen is a public-law corporation
          (<em>Körperschaft des öffentlichen Rechts</em>) and is legally represented by its President.
          AI-DB is operated by the Chair of Bioinformatics and Systems Biology.
        </p>
        <p>
          Contact:
          <a :href="`mailto:${CONTACT_EMAIL}`">{{ CONTACT_EMAIL }}</a>
        </p>
      </section>

      <!-- ── Privacy Policy ── -->
      <div id="privacy" class="section-group-title">Privacy Policy</div>

      <section class="legal-section">
        <h3>1. Controller</h3>
        <p>
          The controller responsible for data processing within the meaning of
          the General Data Protection Regulation (GDPR) is:
        </p>
        <address>
          President of the<br>
          Justus Liebig University Giessen (JLU)<br>
          Ludwigstraße 23<br>
          35390 Gießen<br>
          Germany
        </address>
        <p>
          Contact:
          <a :href="`mailto:${CONTACT_EMAIL}`">{{ CONTACT_EMAIL }}</a>
        </p>
      </section>

      <section class="legal-section">
        <h3>2. Collection of Personal Data</h3>
        <p>
          AI-DB can be used without registration or a user account. We do not ask
          you for your name, e-mail address, or any other identifying information.
        </p>
        <p>
          When you access AI-DB, the web server automatically processes technical
          connection data that your browser transmits:
        </p>
        <ul>
          <li>IP address of the requesting device</li>
          <li>Date and time of access</li>
          <li>Requested URL and HTTP status code</li>
          <li>Browser type and version (user agent)</li>
        </ul>
        <p>
          Server log files are used solely to operate the service, to detect and
          resolve errors, and to defend against misuse. They are not merged with
          other data sources.
        </p>
      </section>

      <section class="legal-section">
        <h3>3. Submitted Sequences and Analysis Jobs</h3>
        <p>
          Protein sequences you submit are stored as MD5 Hashes together with the resulting
          analysis job and are identified by a random job ID. Anyone who knows a
          job ID can open the corresponding results, so please treat job links
          accordingly. Jobs and their sequences are deleted automatically
          30 days after creation.
        </p>
        <ul>
          <li>
            <strong>Annotation lookup</strong> — sequences are matched by their MD5
            hash against the Bakta database and the AI-DB annotations DB. This
            lookup takes place entirely on our servers.
          </li>
          <li>
            <strong>External annotation services</strong> — sequences without a match
            can optionally be forwarded to external annotation services (Bakta Web API,
            including Baktfold, and Psos). Forwarding happens only when you explicitly
            start it via the corresponding button. The affected sequences are then
            processed according to the respective service's own terms and privacy policy.
          </li>
          <li>
            <strong>Contribution to the AI-DB annotations DB</strong> — by default,
            annotations obtained for previously unmatched sequences are added to the
            AI-DB annotations DB so that future lookups by all users benefit from them.
            Only the MD5 hash of each sequence and the derived annotation are stored
            permanently — not the sequence itself and no reference to your job. You can
            disable this contribution at any time in the job view.
          </li>
        </ul>
      </section>

      <section class="legal-section">
        <h3>4. Browser Storage</h3>
        <p>
          AI-DB does not use cookies. We use your browser's local storage solely to
          remember settings you have made yourself: your colour theme (light or dark),
          dismissed service notices, and whether annotations may be contributed to the
          AI-DB annotations DB. This information never leaves your device and can be
          removed at any time by clearing your browser's site data.
        </p>
      </section>

      <section class="legal-section">
        <h3>5. Third-Party Content</h3>
        <p>
          To display results, AI-DB loads content from external providers directly
          in your browser. Your IP address is transmitted to these providers as a
          technical necessity:
        </p>
        <ul>
          <li>
            <strong>QuickGO REST API</strong> (EMBL-EBI, Hinxton, United Kingdom) —
            resolution of Gene Ontology term labels.
          </li>
          <li>
            <strong>Content delivery network</strong> — loading of the ECharts library
            used for functional statistics charts.
          </li>
          <li>
            <strong>UniRef, UniParc, and NCBI</strong> — availability checks and links
            to external database entries.
          </li>
        </ul>
        <p>
          The United Kingdom is covered by an adequacy decision of the European
          Commission (Art. 45 GDPR).
        </p>
      </section>

      <section class="legal-section">
        <h3>6. Usage Statistics</h3>
        <p>
          To report on the use of AI-DB to our funding bodies, we record aggregated
          monthly figures (e.g. number of jobs and sequences submitted). These figures
          contain no personal data and cannot be traced back to individual users.
        </p>
      </section>

      <section class="legal-section">
        <h3>7. Purpose and Legal Basis of Data Processing</h3>
        <p>
          Data is processed exclusively to provide AI-DB as a research service, to
          ensure its technical operation and security, and to compile aggregated usage
          statistics. As a public university, JLU processes this data in the performance
          of its tasks in the public interest (research and the provision of research
          infrastructure). The legal basis is Art. 6 para. 1 lit. e GDPR in conjunction
          with § 3 of the Hessian Data Protection and Freedom of Information Act (HDSIG).
        </p>
        <p>
          Storing your settings in local storage is strictly necessary to provide the
          function you have requested (§ 25 para. 2 no. 2 TDDDG).
        </p>
      </section>

      <section class="legal-section">
        <h3>8. Storage Period</h3>
        <ul>
          <li><strong>Analysis jobs and submitted sequences</strong> — deleted automatically after 30 days.</li>
          <li><strong>Server log files</strong> — deleted after a short, defined retention period.</li>
          <li><strong>Contributed annotations</strong> (MD5 hash and annotation only) — retained permanently as part of the AI-DB annotations DB.</li>
          <li><strong>Aggregated usage statistics</strong> — retained permanently; they contain no personal data.</li>
        </ul>
      </section>

      <section class="legal-section">
        <h3>9. Your Rights as a Data Subject</h3>
        <p>You have the following rights under the GDPR:</p>
        <ul>
          <li><strong>Right of access</strong> (Art. 15 GDPR) — you may request information about the personal data we hold about you.</li>
          <li><strong>Right to rectification</strong> (Art. 16 GDPR) — you may request correction of inaccurate data.</li>
          <li><strong>Right to erasure</strong> (Art. 17 GDPR) — you may request deletion of your data, subject to legal retention obligations.</li>
          <li><strong>Right to restriction of processing</strong> (Art. 18 GDPR).</li>
          <li><strong>Right to object</strong> (Art. 21 GDPR) — you may object to processing based on Art. 6 para. 1 lit. e GDPR on grounds relating to your particular situation.</li>
          <li><strong>Right to lodge a complaint</strong> (Art. 77 GDPR) — you may lodge a complaint with the Hessian Data Protection Officer (<em>Der Hessische Beauftragte für Datenschutz und Informationsfreiheit</em>).</li>
        </ul>
        <p>
          As AI-DB does not use accounts, we can generally only assign data to you if
          you provide the corresponding job ID with your request.
        </p>
      </section>

      <section class="legal-section">
        <h3>10. Contact for Data Protection Enquiries</h3>
        <p>
          If you have any questions about the collection, processing, or use of
          your personal data, please contact us at:
          <a :href="`mailto:${CONTACT_EMAIL}`">{{ CONTACT_EMAIL }}</a>
        </p>
      </section>

      <section class="legal-section">
        <h3>11. Data Protection Officer</h3>
        <p>The contact details of the official Data Protection Officer at JLU are:</p>
        <address>
          Justus Liebig University Giessen<br>
          Der behördliche Datenschutzbeauftragte<br>
          Ludwigstraße 23<br>
          35390 Gießen<br>
          Tel: 0641 99-12230 / 0641 99-12250<br>
          <a href="mailto:datenschutz@uni-giessen.de">datenschutz@uni-giessen.de</a>
        </address>
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

.section-group-title {
  font-size: 1.35rem;
  font-weight: 700;
  color: var(--color-heading);
  padding: 1.5rem 0 1rem;
  border-top: 2px solid var(--legal-primary);
  margin-top: 1rem;
  opacity: 0.9;
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

.legal-section p,
.legal-section address {
  color: var(--color-text);
  opacity: 0.85;
  font-size: 0.95rem;
  line-height: 1.7;
  margin: 0 0 0.75rem 0;
  font-style: normal;
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