import type { Metadata } from "next";
import Link from "next/link";
import { TerminalBlock } from "@/components/terminal";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { GITHUB_RELEASES, GITHUB_REPO } from "@/lib/utils";

export const metadata: Metadata = {
  title: "Install",
};

export default function InstallPage() {
  return (
    <div className="mx-auto max-w-3xl px-4 py-14 sm:px-6">
      <Badge className="mb-4">Get started</Badge>
      <h1 className="text-3xl tracking-tight text-[var(--ink)] sm:text-4xl">
        Install rBuilder
      </h1>
      <p className="mt-3 text-[var(--body)]">
        Prefer a release binary for day-to-day use. Build from source when you
        need the latest main — and pull Git LFS if you want the default semantic
        embedder.
      </p>

      <section className="mt-10 space-y-3">
        <h2 className="text-lg text-[var(--ink)]">Option A — GitHub Releases</h2>
        <p className="text-sm text-[var(--body)]">
          Download the latest asset for your platform from{" "}
          <a
            href={GITHUB_RELEASES}
            className="text-[var(--ink)] underline"
            target="_blank"
            rel="noreferrer"
          >
            Releases
          </a>
          , put <code className="font-mono text-[var(--body-strong)]">rbuilder</code>{" "}
          on your <code className="font-mono">PATH</code>, then:
        </p>
        <TerminalBlock lines={["rbuilder --version"]} />
        <Button variant="ghost" asChild>
          <a href={GITHUB_RELEASES} target="_blank" rel="noreferrer">
            Open releases
          </a>
        </Button>
      </section>

      <section className="mt-12 space-y-3">
        <h2 className="text-lg text-[var(--ink)]">Option B — Build from source</h2>
        <TerminalBlock
          lines={[
            "git clone https://github.com/sshaaf/rBuilder.git",
            "cd rBuilder",
            "# Optional: default semantic embedder (code-daemon ONNX ~206 MB)",
            "# Skip if you only use: semantic index --embedder vocab|hash",
            "git lfs pull",
            "cargo build --release",
            "./target/release/rbuilder --version",
          ]}
        />
      </section>

      <section className="mt-12 space-y-3">
        <h2 className="text-lg text-[var(--ink)]">First hour</h2>
        <p className="text-sm text-[var(--body)]">
          Use the in-tree{" "}
          <code className="font-mono text-[var(--body-strong)]">
            rbuilder-tests/ecommerce-java
          </code>{" "}
          fixture (canonical walkthrough in the User Guide).
        </p>
        <TerminalBlock
          lines={[
            "cd rbuilder-tests/ecommerce-java",
            "rbuilder discover .",
            "rbuilder -f json gql --macro-name all_functions unused | jq '.count'",
            'rbuilder -f json blast-radius "priceShoppingCart" --depth 2',
          ]}
        />
        <p className="text-sm text-[var(--mute)]">
          Dashboard and migration JSON are opt-in: add{" "}
          <code className="font-mono">--with-dashboard</code> /{" "}
          <code className="font-mono">--export-migration-hints</code>.
        </p>
      </section>

      <section className="mt-12 flex flex-wrap gap-3">
        <Button asChild>
          <Link href="/docs/">Read the docs</Link>
        </Button>
        <Button variant="ghost" asChild>
          <Link href="/demo/">Try demos</Link>
        </Button>
        <Button variant="ghost" asChild>
          <a href={`${GITHUB_REPO}/blob/main/docs/user-guide.md`} target="_blank" rel="noreferrer">
            User Guide on GitHub
          </a>
        </Button>
      </section>
    </div>
  );
}
