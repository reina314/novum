# GitHub Pages deployment

The site is configured for the `docs/` directory with Jekyll and the `minima` theme.

In GitHub repository settings, configure Pages to build from:

```text
Build and deployment
  Source: GitHub Actions
```

or, for a branch-based setup:

```text
Source: Deploy from a branch
Branch: main
Folder: /docs
```

The `_config.yml` already specifies the site URL and `/novum` base URL.

