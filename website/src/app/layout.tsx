import type { Metadata } from "next";
import { DM_Mono, Instrument_Serif, Inter } from "next/font/google";
import { Analytics } from "@/components/analytics";
import { SiteFooter } from "@/components/site-footer";
import { SiteHeader } from "@/components/site-header";
import "./globals.css";

const inter = Inter({
  subsets: ["latin"],
  variable: "--font-sans",
  display: "swap",
});

const instrument = Instrument_Serif({
  weight: "400",
  subsets: ["latin"],
  variable: "--font-serif",
  display: "swap",
});

const dmMono = DM_Mono({
  weight: ["400", "500"],
  subsets: ["latin"],
  variable: "--font-mono",
  display: "swap",
});

export const metadata: Metadata = {
  title: {
    default: "rBuilder — code knowledge graph for AI agents",
    template: "%s · rBuilder",
  },
  description:
    "Open-source code knowledge graph for LLM agents. Index once, query compact JSON — blast radius, GQL, semantic search, migration planning.",
  metadataBase: new URL("https://shaaf.dev/rBuilder"),
  openGraph: {
    title: "rBuilder",
    description:
      "Open-source code knowledge graph for LLM agents — accurate answers, fewer tokens.",
    type: "website",
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="en"
      className={`${inter.variable} ${instrument.variable} ${dmMono.variable} h-full`}
    >
      <body className="flex min-h-full flex-col antialiased">
        <SiteHeader />
        <main className="flex-1">{children}</main>
        <SiteFooter />
        <Analytics />
      </body>
    </html>
  );
}
