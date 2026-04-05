# Interface Design System — airports.report

## Intent

**Who:** Aviation nerds and data enthusiasts who geek out about airports. They read METARs for fun, argue about runway configurations, and have opinions about terminal design. They open this at any hour because airports never close.

**Task:** Explore airport intelligence data — scores, sentiment, delays, routes, operators. Understand how airports compare, what passengers think, where things are improving or declining. On the admin side: manage the data pipeline, import airports, monitor scraping jobs.

**Feel:** Dark, dense, warm-industrial. A control room at night. The confidence of infrastructure — information that *announces* rather than *reports*. Not a generic dashboard; an editorial data product with snarky personality. Like reading an aviation magazine that also happens to have live data.

## Direction

Aviation control room meets editorial data magazine. Dark surfaces with the slight blue-violet tint of instrument panels at night. Monospaced IATA codes treated as first-class typography, not just data. The amber/green semantic language of departure boards for status indicators. Dense information that rewards attention.

## Signature

IATA codes as typographic heroes — `font-mono font-bold` on dark surfaces, the same way departure boards give codes presence. Three-letter codes are the universal language of aviation; they should feel like they belong on a FIDS board, not buried in a table cell.

## Typography

- **Sans:** Geist Variable — the body typeface. Clean, contemporary, good at small sizes for dense data.
- **Display/Headlines:** Space Grotesk (`font-grotesk`) — geometric, slightly quirky. Used for section headings and the logo. Gives editorial personality without being decorative.
- **Mono:** IBM Plex Mono (`font-mono`) — IATA codes, scores, data values, terminal-style displays. The workhorse of the aviation feel. Used generously — not just for "code" but for any data that benefits from the precision aesthetic.

## Color

### Foundation (Dark Mode — primary mode)

Built on shadcn/ui base-nova theme with oklch. Dark mode is the real mode; light mode exists but dark is the identity.

- **Background:** `oklch(0.145 0.005 285)` — near-black with a whisper of blue-violet (hue 285). Not pure black. The tint gives warmth without color.
- **Card/Elevated:** `oklch(0.178 0.005 285)` — one step lighter, same hue. Barely perceptible lift.
- **Muted surfaces:** `oklch(0.215 0.005 285)` — for secondary backgrounds, hover states.
- **Borders:** `oklch(1 0 0 / 8%)` — white at 8% opacity. Disappears when you're not looking, findable when you need structure.
- **Input borders:** `oklch(1 0 0 / 10%)` — slightly more present than layout borders.

### Text Hierarchy

- **Primary:** `oklch(0.985 0 0)` — near-white. Headlines, IATA codes, key values.
- **Secondary:** `oklch(0.716 0.01 285)` — for supporting text with slight blue tint.
- **Muted:** `oklch(0.553 0.01 285)` — metadata, timestamps, labels.

### Semantic / Status Colors

Used consistently across both public pages and admin:

- **Green** (`green-400/500/600`) — success, recent data, positive sentiment, healthy scores (70+)
- **Yellow** (`yellow-400/500`) — stale data, warning, neutral sentiment, mid scores (40-69)
- **Red** (`red-400/500`, destructive) — failed, negative sentiment, poor scores (<40)
- **Blue** (`blue-400`) — scored indicator, informational
- **Amber** — departure-board accent for active/running states

### Score Colors (from `utils/scoring.ts`)

Scores use a green-yellow-red gradient mapped to 0-100:
- `>=70`: green shades
- `40-69`: yellow/amber shades
- `<40`: red shades

Applied via `scoreColor()` for text and `scoreBg()` for backgrounds.

## Depth Strategy

**Borders-only** in dark mode. No drop shadows. Hierarchy comes from surface color shifts (the oklch lightness progression) and border opacity. This fits the instrument-panel aesthetic — flat, precise, no decorative depth.

- Layout borders: `border-border` (8% white)
- Card separation: `border` class on cards
- Focus rings: `ring` token
- No box-shadows on cards or containers

## Spacing

Base unit: **4px (Tailwind default)**. Common patterns:
- Micro: `gap-1` (4px) — icon-to-text
- Component: `gap-2` to `gap-3` (8-12px) — within cards, between buttons
- Section: `gap-4` to `gap-6` (16-24px) — between card groups
- Major: `gap-8` to `gap-12` (32-48px) — page sections

## Border Radius

`--radius: 0.625rem` (10px) as base. Scale:
- `radius-sm`: 6px — inputs, buttons, badges
- `radius-md`: 8px — cards, dialogs
- `radius-lg`: 10px — larger containers

Slightly rounded but not bubbly. Technical, not playful.

## Component Patterns

### Public Pages (airport detail, home, operators)

- Dense editorial layouts. Data presented with narrative context, not just numbers.
- Score bars with color-coded fills and monospaced values.
- Sections introduced with Space Grotesk headings.
- Recharts for visualization with custom dark tooltips (`utils/styles.ts`).
- IATA codes always `font-mono font-bold`.

### Admin Pages

- shadcn/ui component library: Button, Card, Table, Dialog, Badge, Input, Textarea, Checkbox.
- Tables for data listing with inline actions.
- Filter bars: text search + dropdown filters + sort selectors.
- Status badges: green (enabled/success), red (failed/destructive), yellow (stale), zinc (never).
- Source indicators: colored dots with hover tooltips showing per-source status.
- Log terminal: dark monospace scrolling log display.

### Navigation

- Public: top nav bar with logo, search (cmdk), and page links. Hidden on admin pages.
- Admin: `AdminLayout` component with breadcrumb-style title and action buttons. Navigation via sidebar or tab links at `/admin/*`.

## Data Display Conventions

- Numbers formatted with `fmt()` and `fmtM()` from `utils/format.ts`
- Percentages include the `%` symbol
- Dates in relative format where recent, absolute where historical
- Reviews are **anonymous** — never show reviewer names or photos
- Scores on 0-100 scale, sub-scores on 1-5 scale
- Score thresholds for color: >=3.5 green, 2.5-3.5 yellow, <2.5 red (sub-scores)

## Files

- `web/app/styles.css` — CSS custom properties, font imports, dark mode tokens
- `web/app/utils/scoring.ts` — `scoreColor()`, `scoreBg()` functions
- `web/app/utils/format.ts` — `fmt()`, `fmtM()`, `cleanCity()` formatters
- `web/app/utils/constants.ts` — score/sentiment explanations, pipeline source names
- `web/app/utils/styles.ts` — shared chart tooltip styles
- `web/app/components/ui/*` — shadcn/ui base components
