export function fmt(n: number | string | null | undefined): string {
  if (n == null) return "—";
  const num = typeof n === "string" ? parseFloat(n) : n;
  if (isNaN(num)) return "—";
  return num.toLocaleString("en-US");
}

export function cleanCity(city: string): string {
  const parts = city.split(", ");
  if (parts.length === 2 && parts[0] === parts[1]) return parts[0];
  return city;
}

export function fmtM(n: number | string | null | undefined): string {
  if (n == null) return "—";
  const num = typeof n === "string" ? parseFloat(n) : n;
  if (isNaN(num)) return "—";
  if (num >= 1_000_000_000) return `${(num / 1_000_000_000).toFixed(1)}B`;
  if (num >= 1_000_000) return `${(num / 1_000_000).toFixed(1)}M`;
  if (num >= 1_000) return `${(num / 1_000).toFixed(0)}K`;
  return fmt(num);
}
