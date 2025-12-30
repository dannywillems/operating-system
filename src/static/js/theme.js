// Theme switching with localStorage persistence and system preference detection

(function() {
    'use strict';

    const STORAGE_KEY = 'theme';
    const DARK = 'dark';
    const LIGHT = 'light';

    // Get theme from localStorage or system preference
    function getPreferredTheme() {
        const stored = localStorage.getItem(STORAGE_KEY);
        if (stored) {
            return stored;
        }
        // Check system preference
        if (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) {
            return DARK;
        }
        return LIGHT;
    }

    // Apply theme to document
    function setTheme(theme) {
        document.documentElement.setAttribute('data-theme', theme);
        localStorage.setItem(STORAGE_KEY, theme);
        updateToggleIcon(theme);
    }

    // Update toggle button icon
    function updateToggleIcon(theme) {
        const toggle = document.getElementById('theme-toggle');
        if (toggle) {
            const icon = toggle.querySelector('i');
            if (icon) {
                icon.className = theme === DARK ? 'bi bi-sun-fill' : 'bi bi-moon-fill';
            }
        }
    }

    // Toggle between light and dark
    function toggleTheme() {
        const current = document.documentElement.getAttribute('data-theme') || LIGHT;
        const next = current === DARK ? LIGHT : DARK;
        setTheme(next);
    }

    // Initialize theme on page load
    function init() {
        // Apply theme immediately (before DOM ready to prevent flash)
        setTheme(getPreferredTheme());

        // Set up toggle button when DOM is ready
        document.addEventListener('DOMContentLoaded', function() {
            const toggle = document.getElementById('theme-toggle');
            if (toggle) {
                toggle.addEventListener('click', toggleTheme);
            }
            // Update icon after DOM ready
            updateToggleIcon(getPreferredTheme());
        });

        // Listen for system preference changes
        if (window.matchMedia) {
            window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', function(e) {
                // Only auto-switch if user hasn't manually set a preference
                if (!localStorage.getItem(STORAGE_KEY)) {
                    setTheme(e.matches ? DARK : LIGHT);
                }
            });
        }
    }

    // Run initialization
    init();
})();
