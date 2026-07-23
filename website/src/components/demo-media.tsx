import { withBase, cn } from "@/lib/utils";

type DemoMediaProps = {
  kind: "cli" | "dashboard";
  className?: string;
  caption?: string;
  /** Prefer looping GIF in hero; MP4 with controls elsewhere. */
  preferGif?: boolean;
};

const assets = {
  cli: {
    mp4: "/demos/user-guide-cli.mp4",
    gif: "/demos/user-guide-cli.gif",
    label: "CLI walkthrough (VHS)",
    blurb:
      "Recorded from docs/videos/user-guide-cli.tape — discover, GQL, blast-radius, CPG, semantic.",
    alt: "rBuilder CLI demo: discover, query, blast-radius, and semantic search",
  },
  dashboard: {
    mp4: "/demos/feature-demo.mp4",
    gif: null as string | null,
    label: "Dashboard tour",
    blurb: "Tab montage over ecommerce-java after discover --with-dashboard.",
    alt: "rBuilder dashboard feature demo across main tabs",
  },
} as const;

export function DemoMedia({
  kind,
  className,
  caption,
  preferGif = false,
}: DemoMediaProps) {
  const asset = assets[kind];
  const mp4 = withBase(asset.mp4);
  const gif = asset.gif ? withBase(asset.gif) : null;
  const useGif = preferGif && gif;

  return (
    <figure className={cn("space-y-3", className)}>
      <div className="overflow-hidden rounded-[4px] border border-[var(--hairline)] bg-[#1f1c19] shadow-[0_16px_48px_rgba(0,0,0,0.4)]">
        <div className="flex items-center gap-1.5 border-b border-[var(--hairline)] px-3 py-2">
          <span className="h-2.5 w-2.5 rounded-full bg-[#5c534c]" aria-hidden />
          <span className="h-2.5 w-2.5 rounded-full bg-[#5c534c]" aria-hidden />
          <span className="h-2.5 w-2.5 rounded-full bg-[#5c534c]" aria-hidden />
          <span className="ml-2 font-mono text-[11px] text-[var(--mute)]">
            {asset.label}
          </span>
        </div>
        {useGif ? (
          // eslint-disable-next-line @next/next/no-img-element
          <img
            src={gif}
            alt={asset.alt}
            className="block h-auto w-full"
            loading="eager"
          />
        ) : (
          <video
            className="block h-auto w-full"
            controls
            playsInline
            preload="metadata"
            aria-label={asset.alt}
          >
            <source src={mp4} type="video/mp4" />
          </video>
        )}
      </div>
      <figcaption className="text-sm text-[var(--mute)]">
        {caption ?? asset.blurb}
      </figcaption>
    </figure>
  );
}
