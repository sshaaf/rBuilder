import Link from "next/link";
import { ArrowRight, GitFork, Star } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { CommandBar } from "@/components/command-bar";
import { DemoMedia } from "@/components/demo-media";
import { TerminalBlock } from "@/components/terminal";
import { GITHUB_REPO } from "@/lib/utils";

const pillars = [
  {
    title: "Index once",
    body: "discover builds a rich graph with reachability caches. Agents stop dumping whole trees into context.",
  },
  {
    title: "Query facts",
    body: "blast-radius, GQL, metrics, semantic, and CPG return compact -f json — deterministic structure, not guesses.",
  },
  {
    title: "Ship safer changes",
    body: "Migration hints, policy check, and agent recipes for impact-aware refactors in open source and monorepos.",
  },
];

export default function HomePage() {
  return (
    <>
      <section className="relative overflow-hidden border-b border-[var(--hairline)]">
        <div
          className="pointer-events-none absolute inset-0 opacity-[0.35]"
          style={{
            backgroundImage:
              "radial-gradient(ellipse 80% 50% at 50% -20%, #4a433c 0%, transparent 55%)",
          }}
        />
        <div className="relative mx-auto max-w-6xl px-4 pb-16 pt-16 sm:px-6 sm:pt-24">
          <Badge className="mb-6">Open source · MIT · Rust</Badge>
          <h1 className="max-w-3xl text-4xl font-normal tracking-tight text-[var(--ink)] sm:text-5xl sm:leading-[1.1]">
            A code knowledge graph{" "}
            <em className="font-[family-name:var(--font-serif)] not-italic text-[var(--body-strong)]">
              built for agents
            </em>
          </h1>
          <p className="mt-5 max-w-xl text-lg text-[var(--body)]">
            rBuilder indexes your repository once, then answers reachability and
            structure questions in compact JSON — so coding agents use fewer
            tokens and make fewer confident mistakes.
          </p>
          <div className="mt-8 flex flex-wrap items-center gap-3">
            <Button size="lg" asChild>
              <Link href="/install/">
                Install <ArrowRight className="h-4 w-4" />
              </Link>
            </Button>
            <Button variant="ghost" size="lg" asChild>
              <a href={GITHUB_REPO} target="_blank" rel="noreferrer">
                <Star className="h-4 w-4" /> Star on GitHub
              </a>
            </Button>
            <Button variant="link" asChild>
              <Link href="/demo/">Watch demos</Link>
            </Button>
          </div>
          <div className="mt-12 max-w-3xl">
            <CommandBar />
          </div>
          <div className="mt-10 max-w-4xl">
            <DemoMedia
              kind="cli"
              preferGif
              caption="VHS terminal demo — same path as the User Guide first hour (ecommerce-java)."
            />
          </div>
        </div>
      </section>

      <section className="mx-auto max-w-6xl px-4 py-16 sm:px-6">
        <div className="grid gap-8 md:grid-cols-3">
          {pillars.map((p) => (
            <div key={p.title} className="space-y-2">
              <h2 className="text-base font-medium text-[var(--ink)]">
                {p.title}
              </h2>
              <p className="text-sm leading-relaxed text-[var(--body)]">
                {p.body}
              </p>
            </div>
          ))}
        </div>
      </section>

      <section className="border-y border-[var(--hairline)] bg-[var(--canvas-soft)]/40">
        <div className="mx-auto grid max-w-6xl gap-10 px-4 py-16 sm:px-6 lg:grid-cols-2 lg:items-center">
          <div className="space-y-4">
            <h2 className="text-2xl tracking-tight text-[var(--ink)] sm:text-3xl">
              From prompt → graph facts → edit
            </h2>
            <p className="text-[var(--body)]">
              Drop{" "}
              <Link href="/agents/" className="text-[var(--ink)] underline">
                AGENTS.md
              </Link>{" "}
              into your agent workflow. The model calls rBuilder instead of
              grepping blindly — then reasons on structured impact.
            </p>
            <Button variant="ghost" asChild>
              <Link href="/agents/">
                Agent guide <ArrowRight className="h-4 w-4" />
              </Link>
            </Button>
          </div>
          <TerminalBlock
            lines={[
              "rbuilder discover .",
              'rbuilder -f json semantic query "checkout flow" --limit 5',
              'rbuilder -f json blast-radius "priceShoppingCart" --depth 2',
            ]}
          />
        </div>
      </section>

      <section className="mx-auto max-w-6xl px-4 py-16 sm:px-6">
        <div className="flex flex-col gap-6 rounded-[4px] border border-[var(--hairline)] bg-[var(--canvas-soft)] p-8 sm:flex-row sm:items-center sm:justify-between">
          <div className="space-y-2">
            <h2 className="text-xl text-[var(--ink)]">Help grow the project</h2>
            <p className="max-w-lg text-sm text-[var(--body)]">
              Star the repo, try the ecommerce-java fixture, open issues, and
              share agent recipes. Adoption is the product.
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            <Button asChild>
              <a href={GITHUB_REPO} target="_blank" rel="noreferrer">
                <Star className="h-4 w-4" /> Star
              </a>
            </Button>
            <Button variant="ghost" asChild>
              <a href={`${GITHUB_REPO}/fork`} target="_blank" rel="noreferrer">
                <GitFork className="h-4 w-4" /> Fork
              </a>
            </Button>
            <Button variant="ghost" asChild>
              <Link href="/community/">Community</Link>
            </Button>
          </div>
        </div>
      </section>
    </>
  );
}
