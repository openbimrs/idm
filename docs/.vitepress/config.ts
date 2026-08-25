import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'openbim-idm',
  description: 'Lossless ISO 29481-3 idmXML tooling for Rust and Python',
  lang: 'en-US',
  base: '/idm/',
  cleanUrls: true,
  lastUpdated: true,
  sitemap: { hostname: 'https://openbimrs.github.io/idm/' },
  head: [
    ['meta', { name: 'theme-color', content: '#12736f' }],
    ['meta', { name: 'robots', content: 'index,follow' }],
  ],
  themeConfig: {
    logo: '/logo.svg',
    siteTitle: 'openbim-idm',
    nav: [
      { text: 'Guide', link: '/guide/getting-started' },
      { text: 'API', link: '/api/rust' },
      { text: 'Architecture', link: '/architecture/' },
      { text: 'Security', link: '/security' },
      { text: 'Changelog', link: '/project/changelog' },
    ],
    sidebar: {
      '/guide/': [
        {
          text: 'Guide',
          items: [
            { text: 'Getting started', link: '/guide/getting-started' },
            { text: 'Schema validation', link: '/guide/schema-validation' },
          ],
        },
      ],
      '/api/': [
        {
          text: 'API',
          items: [
            { text: 'Rust', link: '/api/rust' },
            { text: 'Python', link: '/api/python' },
            { text: 'Command line', link: '/api/cli' },
          ],
        },
      ],
      '/architecture/': [
        {
          text: 'Architecture',
          items: [
            { text: 'System design', link: '/architecture/' },
            { text: 'Canonical and alias crates', link: '/architecture/canonical-alias' },
          ],
        },
      ],
      '/': [
        {
          text: 'Project',
          items: [
            { text: 'Security', link: '/security' },
            { text: 'Provenance', link: '/provenance' },
            { text: 'Changelog', link: '/project/changelog' },
          ],
        },
      ],
    },
    socialLinks: [{ icon: 'github', link: 'https://github.com/openbimrs/idm' }],
    editLink: {
      pattern: 'https://github.com/openbimrs/idm/edit/main/docs/:path',
      text: 'Edit this page on GitHub',
    },
    search: { provider: 'local' },
    footer: {
      message: 'Implementation licensed under MIT. Standards material is not redistributed.',
      copyright: 'Copyright © 2026 openbimrs contributors',
    },
  },
})
