import type { Metadata } from "next";
import Link from "next/link";
import { Badge } from "@/components/ui/badge";
import { DemoMedia } from "@/components/demo-media";
import { DemoPlayground } from "@/components/demo-playground";
import { GITHUB_REPO } from "@/lib/utils";

export const metadata: Metadata = {
  title: "Demo",
};

export default function DemoPage() {
  return (
    <div className="mx-auto max-w-6xl px-4 py-14 sm:px-6">
      <Badge className="mb-4">Recorded + interactive</Badge>
      <h1 className="text-3xl tracking-tight text-[var(--ink)] sm:text-4xl">
        See rBuilder in action
      </h1>
      <p className="mt-3 max-w-2xl text-[var(--body)]">
        VHS terminal recording from{" "}
        <a
          href={`${GITHUB_REPO}/blob/main/docs/videos/user-guide-cli.tape`}
          className="text-[var(--ink)] underline"
          target="_blank"
          rel="noreferrer"
        >
          user-guide-cli.tape
        </a>
        , plus interactive agent scenarios. Prefer the ecommerce-java fixture
        when you run commands locally.
      </p>

      <div className="mt-10 space-y-14">
        <DemoMedia
          kind="cli"
          preferGif
          className="mx-auto max-w-4xl"
          caption="CLI (VHS) — discover → GQL → communities → blast → CPG → semantic."
        />
        <DemoMedia
          kind="dashboard"
          className="mx-auto max-w-4xl"
          caption="Dashboard montage — Graph, Search, CFG, Blast, Migration, and more."
        />
      </div>

      <p className="mt-4 text-sm text-[var(--mute)]">
        Prefer MP4 with captions?{" "}
        <Link href="/demos/user-guide-cli.mp4" className="underline">
          CLI mp4
        </Link>{" "}
        ·{" "}
        <Link href="/demos/feature-demo.mp4" className="underline">
          Dashboard mp4
        </Link>
        . Regenerate via{" "}
        <code className="font-mono text-[var(--body)]">
          docs/videos/record-*.sh
        </code>
        .
      </p>

      <h2 className="mt-16 text-2xl tracking-tight text-[var(--ink)]">
        Agent skill scenarios
      </h2>
      <p className="mt-2 max-w-2xl text-sm text-[var(--body)]">
        Prompt → tool call → schema-aligned JSON → reasoning. Commands match the
        live CLI.
      </p>
      <div className="mt-8">
        <DemoPlayground />
      </div>
    </div>
  );
}
