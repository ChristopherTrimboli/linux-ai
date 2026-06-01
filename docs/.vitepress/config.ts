import { defineConfig } from "vitepress";

const repo = "https://github.com/ChristopherTrimboli/linux-ai";

// https://vitepress.dev/reference/site-config
export default defineConfig({
  title: "Linux AI",
  description:
    "A Linux-first AI chat app with tool-based access to your computer and a matching CLI. Free and open source.",
  // Project site served from https://<user>.github.io/linux-ai/
  base: "/linux-ai/",
  lastUpdated: true,
  cleanUrls: true,
  head: [["link", { rel: "icon", href: "/linux-ai/favicon.png" }]],
  themeConfig: {
    logo: "/favicon.png",
    nav: [
      { text: "Guide", link: "/guide/getting-started" },
      { text: "CLI", link: "/guide/cli" },
      { text: "Releases", link: "/guide/releases" },
    ],
    sidebar: [
      {
        text: "Introduction",
        items: [
          { text: "What it is", link: "/guide/getting-started" },
          { text: "Installation", link: "/guide/installation" },
        ],
      },
      {
        text: "Usage",
        items: [
          { text: "CLI", link: "/guide/cli" },
          { text: "Voice input", link: "/guide/voice" },
          { text: "Configuration & secrets", link: "/guide/configuration" },
          { text: "Security model", link: "/guide/security" },
        ],
      },
    {
      text: "Distribution",
      items: [
        { text: "Install packages", link: "/guide/installation" },
        { text: "Snap package", link: "/guide/snap" },
        { text: "Flatpak", link: "/guide/flatpak" },
        { text: "Arch (AUR)", link: "/guide/aur" },
        { text: "Releases & versioning", link: "/guide/releases" },
      ],
    },
      {
        text: "Project",
        items: [{ text: "Contributing", link: "/guide/contributing" }],
      },
    ],
    socialLinks: [{ icon: "github", link: repo }],
    search: { provider: "local" },
    editLink: {
      pattern: `${repo}/edit/main/docs/:path`,
      text: "Edit this page on GitHub",
    },
    footer: {
      message: "Released under the MIT License.",
      copyright: `Copyright © ${new Date().getFullYear()} Linux AI contributors`,
    },
  },
});
