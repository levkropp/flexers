#!/bin/bash
# Test script to verify Jekyll site builds correctly

set -e

echo "Testing Jekyll site build..."

# Check for required files
echo "✓ Checking required files..."
if [ ! -f "Gemfile" ]; then
    echo "❌ Gemfile not found!"
    exit 1
fi

if [ ! -f "Gemfile.lock" ]; then
    echo "❌ Gemfile.lock not found!"
    exit 1
fi

if [ ! -f "_config.yml" ]; then
    echo "❌ _config.yml not found!"
    exit 1
fi

if [ ! -f ".ruby-version" ]; then
    echo "❌ .ruby-version not found!"
    exit 1
fi

echo "✓ All required files present"

# Check Ruby version
REQUIRED_VERSION=$(cat .ruby-version)
if command -v ruby &> /dev/null; then
    CURRENT_VERSION=$(ruby -v | cut -d' ' -f2)
    echo "✓ Ruby version: $CURRENT_VERSION (required: $REQUIRED_VERSION)"
else
    echo "⚠ Ruby not installed, skipping version check"
fi

# Check for bundle
if ! command -v bundle &> /dev/null; then
    echo "❌ Bundler not installed!"
    echo "Install with: gem install bundler"
    exit 1
fi

echo "✓ Bundler is installed"

# Install dependencies
echo "Installing dependencies..."
bundle install --quiet

# Build the site
echo "Building Jekyll site..."
JEKYLL_ENV=production bundle exec jekyll build --baseurl "/flexers"

# Check build output
if [ -d "_site" ]; then
    echo "✓ Build successful!"
    echo "✓ Output directory: _site"
    echo "✓ Files generated: $(find _site -type f | wc -l)"
else
    echo "❌ Build failed - _site directory not created!"
    exit 1
fi

# Check for index.html
if [ -f "_site/index.html" ]; then
    echo "✓ Index page generated"
else
    echo "❌ Index page not generated!"
    exit 1
fi

# Clean up (optional)
# rm -rf _site

echo ""
echo "✅ All tests passed! Site builds successfully."
echo ""
echo "To serve locally:"
echo "  bundle exec jekyll serve"
echo ""
