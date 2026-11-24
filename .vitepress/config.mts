import { defineConfig } from "vitepress";
import llmstxt from "vitepress-plugin-llms";

export default defineConfig({
  vite: {
    plugins: [llmstxt()],
  },
  title: "lampy",
  description: "telemetry expertise — OTEL port, OTLP impl, and the org env contract",
  base: "/lampy/",
  srcExclude: ["README.md"],
  ignoreDeadLinks: [/\/api\//],
  themeConfig: {
    search: { provider: "local" },
    nav: [
      { text: "9aia", link: "https://9aia.com", target: "_blank" },
      { text: "Tutorials", link: "/tutorials/getting-started" },
      { text: "How-to", link: "/how-to/" },
      { text: "Explanation", link: "/explanation/architecture" },
      { text: "Reference", link: "/reference/" },
      {
        text: "LLMs",
        items: [
          { text: "llms.txt", link: "https://9aia.com/lampy/llms.txt", target: "_blank" },
          {
            text: "llms-full.txt",
            link: "https://9aia.com/lampy/llms-full.txt",
            target: "_blank",
          },
        ],
      },
    ],
    sidebar: [
      {
        text: "Tutorials",
        items: [{ text: "Getting started", link: "/tutorials/getting-started" }],
      },
      {
        text: "How-to",
        items: [
          { text: "Overview", link: "/how-to/" },
          { text: "Emit OTLP traces", link: "/how-to/emit-otlp-traces" },
          { text: "Add a legacy env alias", link: "/how-to/add-env-alias" },
          {
            text: "Write Prometheus textfile",
            link: "/how-to/write-prometheus-textfile",
          },
        ],
      },
      {
        text: "Explanation",
        items: [
          { text: "Overview", link: "/explanation/" },
          { text: "Architecture", link: "/explanation/architecture" },
          { text: "Concepts", link: "/explanation/concepts" },
        ],
      },
      {
        text: "Reference",
        items: [
          { text: "Overview", link: "/reference/" },
          { text: "Crates", link: "/reference/crates" },
        ],
      },
    ],
    socialLinks: [{ icon: "github", link: "https://github.com/9aia/lampy" }],
  },
});
