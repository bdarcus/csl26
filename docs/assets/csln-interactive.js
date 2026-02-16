/*
 * CSLN Interactive Behaviors
 * SPDX-License-Identifier: MPL-2.0
 */

(function() {
  'use strict';

  const CSLN = {
    init: function() {
      this.setupCitations();
      this.setupBibliography();
      this.setupTooltips();
    },

    setupCitations: function() {
      const citations = document.querySelectorAll('.csln-citation');
      citations.forEach(citation => {
        citation.addEventListener('click', (e) => {
          const refs = citation.getAttribute('data-ref').split(' ');
          if (refs.length > 0) {
            this.scrollToEntry(refs[0]);
            this.highlightEntry(refs[0]);
          }
        });

        citation.addEventListener('mouseenter', () => {
          const refs = citation.getAttribute('data-ref').split(' ');
          refs.forEach(ref => this.highlightEntry(ref, true));
        });

        citation.addEventListener('mouseleave', () => {
          const refs = citation.getAttribute('data-ref').split(' ');
          refs.forEach(ref => this.highlightEntry(ref, false));
        });
      });
    },

    setupBibliography: function() {
      const entries = document.querySelectorAll('.csln-entry');
      entries.forEach(entry => {
        entry.addEventListener('mouseenter', () => {
          const id = entry.id.replace('ref-', '');
          this.highlightCitations(id, true);
        });

        entry.addEventListener('mouseleave', () => {
          const id = entry.id.replace('ref-', '');
          this.highlightCitations(id, false);
        });
      });
    },

    scrollToEntry: function(id) {
      const target = document.getElementById('ref-' + id);
      if (target) {
        target.scrollIntoView({ behavior: 'smooth', block: 'center' });
        // Update URL fragment without jumping
        history.replaceState(null, null, '#ref-' + id);
      }
    },

    highlightEntry: function(id, active = true) {
      const entry = document.getElementById('ref-' + id);
      if (entry) {
        if (active) {
          entry.classList.add('is-highlighted');
        } else {
          entry.classList.remove('is-highlighted');
        }
      }
    },

    highlightCitations: function(refId, active = true) {
      const citations = document.querySelectorAll(`.csln-citation[data-ref~="${refId}"]`);
      citations.forEach(citation => {
        if (active) {
          citation.classList.add('is-highlighted');
        } else {
          citation.classList.remove('is-highlighted');
        }
      });
    },

    setupTooltips: function() {
      const tooltip = document.createElement('div');
      tooltip.className = 'csln-tooltip';
      document.body.appendChild(tooltip);

      const citations = document.querySelectorAll('.csln-citation');
      citations.forEach(citation => {
        citation.addEventListener('mousemove', (e) => {
          const refs = citation.getAttribute('data-ref').split(' ');
          if (refs.length === 0) return;

          // For simplicity in the demo, we use the first reference for the tooltip
          const entry = document.getElementById('ref-' + refs[0]);
          if (!entry) return;

          const author = entry.getAttribute('data-author') || '';
          const year = entry.getAttribute('data-year') || '';
          const title = entry.getAttribute('data-title') || '';

          if (!author && !title) return;

          tooltip.innerHTML = `
            ${author ? `<span class="csln-tooltip-author">${author}${year ? ` (${year})` : ''}</span>` : ''}
            ${title ? `<span class="csln-tooltip-title">${title}</span>` : ''}
          `;

          tooltip.style.left = (e.pageX + 15) + 'px';
          tooltip.style.top = (e.pageY + 15) + 'px';
          tooltip.classList.add('is-visible');
        });

        citation.addEventListener('mouseleave', () => {
          tooltip.classList.remove('is-visible');
        });
      });
    }
  };

  // Initialize when DOM is ready
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', () => CSLN.init());
  } else {
    CSLN.init();
  }

  // Export to window
  window.CSLN = CSLN;
})();
