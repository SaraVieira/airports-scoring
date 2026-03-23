import { RouteRow } from "./types";

export function routeDisplayName(r: RouteRow): string {
  return (
    r.destination?.name ??
    r.destinationIata ??
    r.destinationIcao ??
    "Unknown"
  );
}

export function routeIata(r: RouteRow): string | null {
  return r.destinationIata ?? null;
}

export function routeCountry(r: RouteRow): string {
  return r.destination?.countryCode ?? "Unknown";
}

// Module-level Record for O(1) country-to-region lookups
const countryToRegion: Record<string, string> = {};

for (const code of [
  "AT","BE","BG","HR","CY","CZ","DK","EE","FI","FR","DE","GR","HU","IE","IT",
  "LV","LT","LU","MT","NL","PL","PT","RO","SK","SI","ES","SE","GB","NO","CH",
  "IS","AL","BA","ME","MK","RS","XK","UA","MD","BY",
]) {
  countryToRegion[code] = "Europe";
}

for (const code of [
  "DZ","AO","BJ","BW","BF","BI","CV","CM","CF","TD","KM","CD","CG","CI","DJ",
  "EG","GQ","ER","SZ","ET","GA","GM","GH","GN","GW","KE","LS","LR","LY","MG",
  "MW","ML","MR","MU","MA","MZ","NA","NE","NG","RW","ST","SN","SC","SL","SO",
  "ZA","SS","SD","TZ","TG","TN","UG","ZM","ZW",
]) {
  countryToRegion[code] = "Africa";
}

for (const code of [
  "AE","BH","IL","IQ","IR","JO","KW","LB","OM","PS","QA","SA","SY","TR","YE",
]) {
  countryToRegion[code] = "Middle East";
}

for (const code of [
  "AF","AM","AZ","BD","BT","BN","KH","CN","GE","IN","ID","JP","KZ","KG","LA",
  "MY","MV","MN","MM","NP","KP","PK","PH","RU","SG","KR","LK","TW","TJ","TH",
  "TL","TM","UZ","VN",
]) {
  countryToRegion[code] = "Asia";
}

for (const code of [
  "US","CA","MX","BR","AR","CL","CO","PE","VE","EC","BO","PY","UY","GY","SR",
  "CR","PA","CU","DO","HT","JM","TT","BS","BB","GT","HN","SV","NI","BZ","PR",
]) {
  countryToRegion[code] = "Americas";
}

// Simple continent mapping by country code
export function routeRegion(r: RouteRow): string {
  const country = routeCountry(r);
  return countryToRegion[country] ?? "Other";
}
