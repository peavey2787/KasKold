// KasKold Companion Web — stable application entry point
// Copyright (C) 2025-2026 KasSigner Project — GPL-3.0
import { bindShellControls } from './app/shell_controls.js';
import './mobile/native_adaptations.js';
bindShellControls();
async function boot() {
    try {
        const { startApplication } = await import('./app/bootstrap.js');
        await startApplication();
    } catch (error) {
        console.error('Companion init failed:', error);
        const status = document.getElementById('companion-startup-status');
        if (status && status.dataset.state !== 'error') {
            status.textContent = 'Companion could not load its application modules. Build the web app and serve the web directory over HTTP.';
            status.dataset.state = 'error';
        }
    }
}
boot();
