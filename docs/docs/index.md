---
layout: default
title: Documentation
---

<div class="docs-container">
    <div class="docs-header">
        <h1>Documentation</h1>
        <p class="docs-subtitle">Everything you need to understand and use Flexers</p>
    </div>

    <div class="docs-grid">
        <div class="doc-card">
            <h2>🚀 Getting Started</h2>
            <p>Quick introduction to Flexers and how to integrate it into your project.</p>
            <ul>
                <li>What is Flexers?</li>
                <li>Installation</li>
                <li>Your first emulator instance</li>
            </ul>
            <span class="coming-soon">Coming Soon</span>
        </div>

        <div class="doc-card">
            <h2>🏗️ Architecture</h2>
            <p>Deep dive into the emulator's internal design and optimization strategies.</p>
            <ul>
                <li>CPU state layout</li>
                <li>Memory subsystem</li>
                <li>Instruction dispatch</li>
                <li>Performance optimizations</li>
            </ul>
            <a href="{{ '/blog/2026/03/20/phase-1-complete/' | relative_url }}" class="doc-link">Read Phase 1 Blog Post →</a>
        </div>

        <div class="doc-card">
            <h2>📚 API Reference</h2>
            <p>Complete API documentation for all Flexers crates.</p>
            <ul>
                <li><code>flexers-core</code> - CPU & memory</li>
                <li><code>flexers-periph</code> - Peripherals</li>
                <li><code>flexers-stubs</code> - ROM stubs</li>
                <li><code>flexers-session</code> - High-level API</li>
            </ul>
            <span class="coming-soon">Coming Soon</span>
        </div>

        <div class="doc-card">
            <h2>🔧 Integration Guide</h2>
            <p>How to integrate Flexers into your Rust application.</p>
            <ul>
                <li>Adding Flexers as a dependency</li>
                <li>Loading firmware binaries</li>
                <li>Running the emulator</li>
                <li>Handling peripherals</li>
            </ul>
            <span class="coming-soon">Coming Soon</span>
        </div>

        <div class="doc-card">
            <h2>🧪 Testing</h2>
            <p>Writing tests for ESP32 firmware using Flexers.</p>
            <ul>
                <li>Unit testing instructions</li>
                <li>Integration tests</li>
                <li>Differential testing</li>
            </ul>
            <span class="coming-soon">Coming Soon</span>
        </div>

        <div class="doc-card">
            <h2>🎯 Roadmap</h2>
            <p>Development timeline and future features.</p>
            <ul>
                <li>Phase 1: Core CPU & Memory ✓ 85%</li>
                <li>Phase 2: Peripherals & I/O</li>
                <li>Phase 3: ROM Stubs</li>
                <li>Phase 4: Display Integration</li>
                <li>Phase 5: Cyders Integration</li>
            </ul>
            <a href="{{ '/#roadmap' | relative_url }}" class="doc-link">View Full Roadmap →</a>
        </div>
    </div>

    <div class="docs-cta">
        <h2>Need Help?</h2>
        <p>Found a bug or have a question?</p>
        <div class="cta-buttons">
            <a href="https://github.com/levkropp/flexers/issues" class="btn btn-primary">Open an Issue</a>
            <a href="https://github.com/levkropp/flexers/discussions" class="btn btn-secondary">Start a Discussion</a>
        </div>
    </div>
</div>

<style>
.docs-container {
    max-width: 1000px;
    margin: 0 auto;
    padding: 5rem 2rem;
}

.docs-header {
    text-align: center;
    margin-bottom: 4rem;
}

.docs-header h1 {
    font-size: 3rem;
    background: linear-gradient(135deg, var(--accent-blue-bright), var(--accent-gold-bright));
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
    margin-bottom: 1rem;
}

.docs-subtitle {
    color: var(--text-muted);
    font-size: 1.1rem;
}

.docs-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 2rem;
    margin-bottom: 4rem;
}

.doc-card {
    background: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: 12px;
    padding: 2rem;
    transition: all 0.3s;
    position: relative;
}

.doc-card:hover {
    border-color: var(--accent-blue);
    transform: translateY(-4px);
    box-shadow: 0 8px 30px var(--accent-blue-glow);
}

.doc-card h2 {
    color: var(--accent-blue);
    font-size: 1.5rem;
    margin-bottom: 1rem;
}

.doc-card p {
    color: var(--text-muted);
    margin-bottom: 1.5rem;
    line-height: 1.6;
}

.doc-card ul {
    list-style: none;
    padding: 0;
    margin-bottom: 1.5rem;
}

.doc-card li {
    color: var(--text-muted);
    padding: 0.4rem 0;
    font-size: 0.9rem;
}

.doc-card li::before {
    content: "→ ";
    color: var(--accent-blue);
    margin-right: 0.5rem;
}

.doc-card code {
    font-family: 'JetBrains Mono', monospace;
    background: var(--bg-darker);
    padding: 0.2rem 0.4rem;
    border-radius: 4px;
    font-size: 0.85em;
    color: var(--accent-gold);
}

.coming-soon {
    display: inline-block;
    background: var(--accent-gold-glow);
    border: 1px solid var(--accent-gold);
    color: var(--accent-gold);
    padding: 0.3rem 0.8rem;
    border-radius: 16px;
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
}

.doc-link {
    color: var(--accent-blue);
    text-decoration: none;
    font-weight: 500;
    transition: color 0.2s;
}

.doc-link:hover {
    color: var(--accent-blue-bright);
}

.docs-cta {
    text-align: center;
    padding: 3rem;
    background: var(--bg-card);
    border: 1px solid var(--border-accent);
    border-radius: 12px;
}

.docs-cta h2 {
    font-size: 2rem;
    margin-bottom: 0.5rem;
    color: var(--text-primary);
}

.docs-cta p {
    color: var(--text-muted);
    margin-bottom: 2rem;
}
</style>
