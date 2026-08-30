# Publishing the Novum Documentation with GitHub Pages

The `docs/` directory is configured as a Jekyll site using the Just the Docs theme.

## Repository settings

In GitHub:

1. Open **Settings → Pages**.
2. Select **Deploy from a branch**.
3. Choose the `vm` branch and the `/docs` folder.
4. Save the setting.

The configured site base URL is:

```text
https://reina314.github.io/novum/
```

The current `_config.yml` uses `just-the-docs/just-the-docs` as a remote theme so the documentation gets a navigation sidebar, search, hierarchy, and mobile-friendly layout without adding theme source files to this directory.
