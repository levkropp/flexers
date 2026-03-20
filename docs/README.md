# Flexers Documentation Site

This directory contains the Jekyll-based GitHub Pages site for Flexers.

## Local Development

```bash
# Install dependencies
cd docs
bundle install

# Run local server
bundle exec jekyll serve

# Open http://localhost:4000/flexers/
```

## Structure

```
docs/
├── _config.yml           # Jekyll configuration
├── _layouts/             # Page templates
│   ├── default.html      # Base layout with nav/footer
│   └── post.html         # Blog post layout
├── _posts/               # Blog posts (YYYY-MM-DD-title.md)
├── assets/
│   ├── css/main.css      # ESP32-themed styles (black/blue/gold)
│   └── js/circuit.js     # Animated circuit background
├── blog/index.html       # Blog index
├── docs/index.md         # Documentation index
└── index.html            # Landing page
```

## Design

- **Color Scheme**: Black background (#0a0a0f) with blue (#2196F3) and gold (#FFC107) accents
- **Background**: Animated circuit board traces (ESP32-inspired)
- **Typography**: Inter (body), JetBrains Mono (code/headers)

## Adding Blog Posts

Create a new file in `_posts/` with format `YYYY-MM-DD-title.md`:

```markdown
---
layout: post
title: "Your Title"
date: 2026-03-20 12:00:00 -0000
categories: development rust
author: Your Name
excerpt: "Brief description for listings"
---

Your content here...
```

## License

MIT OR Apache-2.0 (same as Flexers)
