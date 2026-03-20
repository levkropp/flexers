# Quick Start: View Flexers Site Locally

## Prerequisites

- **Ruby** 3.0 or higher ([Download](https://www.ruby-lang.org/en/downloads/))
- **Bundler** (comes with Ruby)

## Setup & Run

```bash
# 1. Navigate to docs directory
cd flexers/docs

# 2. Install dependencies (first time only)
bundle install

# 3. Run local server
bundle exec jekyll serve

# 4. Open in browser
# http://localhost:4000/flexers/
```

## Expected Output

```
Configuration file: docs/_config.yml
            Source: docs
       Destination: docs/_site
 Incremental build: disabled. Enable with --incremental
      Generating...
       Jekyll Feed: Generating feed for posts
                    done in 1.234 seconds.
 Auto-regeneration: enabled for 'docs'
    Server address: http://127.0.0.1:4000/flexers/
  Server running... press ctrl-c to stop.
```

## Troubleshooting

### Ruby not installed
**Windows**: Download from [rubyinstaller.org](https://rubyinstaller.org/)
**macOS**: `brew install ruby`
**Linux**: `sudo apt install ruby-full`

### Bundle install fails
```bash
gem install bundler
bundle install
```

### Port 4000 already in use
```bash
bundle exec jekyll serve --port 4001
# Then open http://localhost:4001/flexers/
```

### Changes not reflecting
- Hard refresh: `Ctrl+Shift+R` (Windows/Linux) or `Cmd+Shift+R` (Mac)
- Or clear browser cache

## File Watching

Jekyll automatically rebuilds when you edit files:
- Changes to `*.html`, `*.md`, `*.css` are detected
- Refresh browser to see updates
- Changes to `_config.yml` require restart

## Editing Content

### Edit landing page
`docs/index.html`

### Edit blog post
`docs/_posts/2026-03-20-phase-1-complete.md`

### Edit styles
`docs/assets/css/main.css`

### Edit circuit animation
`docs/assets/js/circuit.js`

## Building for Production

```bash
# Build static site (for manual deployment)
bundle exec jekyll build

# Output in docs/_site/
# Upload _site/ contents to any static host
```

## Live Reload (Optional)

For automatic browser refresh on changes:

```bash
# Install livereload
gem install jekyll-livereload

# Add to Gemfile
echo 'gem "jekyll-livereload"' >> Gemfile

# Serve with livereload
bundle exec jekyll serve --livereload
```

## Common Commands

```bash
# Standard serve
bundle exec jekyll serve

# With drafts
bundle exec jekyll serve --drafts

# Different port
bundle exec jekyll serve --port 4001

# Production build
JEKYLL_ENV=production bundle exec jekyll build

# Clean build cache
bundle exec jekyll clean
```

## Next Steps

1. **View the site** - Navigate to http://localhost:4000/flexers/
2. **Explore pages**:
   - Landing page with features & roadmap
   - Blog → Phase 1 Complete post
   - Docs → Documentation hub
3. **Try editing**:
   - Edit `docs/index.html` and refresh
   - Watch Jekyll rebuild automatically

## Resources

- **Jekyll Docs**: https://jekyllrb.com/docs/
- **Liquid Syntax**: https://shopify.github.io/liquid/
- **Markdown Guide**: https://www.markdownguide.org/

---

**Enjoy exploring Flexers! 🎉**
