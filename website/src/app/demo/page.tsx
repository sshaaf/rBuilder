import type { Metadata } from "next";
import { Badge } from "@/components/ui/badge";
import { DemoPlayground } from "@/components/demo-playground";

export const metadata: Metadata = {
  title: "Demo",
};

export default function DemoPage() {
  return (
    <div className="mx-auto max-w-6xl px-4 py-14 sm:px-6">
      <Badge className="mb-4">Agent skill demos</Badge>
      <h1 className="text-3xl tracking-tight text-[var(--ink)] sm:text-4xl">
        Prompt → rBuilder → facts
      </h1>
      <p className="mt-3 max-w-2xl text-[var(--body)]">
        Scenarios from the verified demo script. Commands match the live CLI;
        JSON samples are illustrative but schema-aligned. Prefer the
        ecommerce-java fixture when you run them locally.
      </p>
      <div className="mt-10">
        <DemoPlayground />
      </div>
    </div>
  );
}
