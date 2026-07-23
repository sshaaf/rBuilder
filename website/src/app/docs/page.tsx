import type { Metadata } from "next";
import Link from "next/link";
import { ArrowUpRight } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { GITHUB_REPO } from "@/lib/utils";

export const metadata: Metadata = {
  title: "Docs",
};

const cards = [
  {
    title: "Introduction",
    blurb: "Concepts — graph, reachability, each capability.",
    href: `${GITHUB_REPO}/blob/main/docs/Introduction.md`,
  },
  {
    title: "User Guide",
    blurb: "Install, ecommerce-java walkthrough, every CLI command.",
    href: `${GITHUB_REPO}/blob/main/docs/user-guide.md`,
  },
  {
    title: "AGENTS.md",
    blurb: "Index once, query with -f json — agent contract.",
    href: `${GITHUB_REPO}/blob/main/AGENTS.md`,
  },
  {
    title: "Agent recipes",
    blurb: "Copy-paste workflows for automation.",
    href: `${GITHUB_REPO}/blob/main/docs/agent-recipes.md`,
  },
  {
    title: "JSON API",
    blurb: "schema_version fields for scripts and CI.",
    href: `${GITHUB_REPO}/blob/main/docs/json-api.md`,
  },
  {
    title: "Dashboard guide",
    blurb: "Browser UI after discover --with-dashboard.",
    href: `${GITHUB_REPO}/blob/main/docs/dashboard-user-guide.md`,
  },
  {
    title: "FAQ",
    blurb: "Discover vs semantic, flags, embedders, exit codes.",
    href: `${GITHUB_REPO}/blob/main/docs/faq.md`,
  },
  {
    title: "Glossary",
    blurb: "Blast, CPG, communities, L_proc, Hamming…",
    href: `${GITHUB_REPO}/blob/main/docs/glossary.md`,
  },
];

export default function DocsPage() {
  return (
    <div className="mx-auto max-w-6xl px-4 py-14 sm:px-6">
      <Badge className="mb-4">Documentation</Badge>
      <h1 className="text-3xl tracking-tight text-[var(--ink)] sm:text-4xl">
        Docs hub
      </h1>
      <p className="mt-3 max-w-2xl text-[var(--body)]">
        Canonical markdown lives in the repository. Start here by persona, then
        open the source on GitHub (always current with main).
      </p>

      <div className="mt-8 flex flex-wrap gap-3 text-sm">
        <Link href="/agents/" className="text-[var(--ink)] underline">
          Agents on this site
        </Link>
        <Link href="/install/" className="text-[var(--ink)] underline">
          Install
        </Link>
        <Link href="/demo/" className="text-[var(--ink)] underline">
          Demos
        </Link>
      </div>

      <div className="mt-10 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {cards.map((c) => (
          <a
            key={c.title}
            href={c.href}
            target="_blank"
            rel="noreferrer"
            className="group flex flex-col rounded-[4px] border border-[var(--hairline)] bg-[var(--canvas-soft)]/50 p-5 transition-colors hover:border-[var(--mute)]"
          >
            <div className="mb-2 flex items-start justify-between gap-2">
              <h2 className="text-base font-medium text-[var(--ink)]">
                {c.title}
              </h2>
              <ArrowUpRight className="h-4 w-4 text-[var(--mute)] group-hover:text-[var(--ink)]" />
            </div>
            <p className="text-sm text-[var(--body)]">{c.blurb}</p>
          </a>
        ))}
      </div>

      <p className="mt-10 text-sm text-[var(--mute)]">
        Full index:{" "}
        <a
          href={`${GITHUB_REPO}/blob/main/docs/README.md`}
          className="text-[var(--body-strong)] underline"
          target="_blank"
          rel="noreferrer"
        >
          docs/README.md
        </a>
        .
      </p>
    </div>
  );
}
