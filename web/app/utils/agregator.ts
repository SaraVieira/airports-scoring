import type { OpsRow } from "~/utils/types";

export function aggregateOps(rows: OpsRow[]) {
  let totalFlights = 0;
  let delayedFlights = 0;
  let totalDelayMin = 0;
  let delayMinCount = 0;
  let weatherMin = 0;
  let carrierMin = 0;
  let atcMin = 0;
  let airportMin = 0;
  let totalAtfmMin = 0;
  let cancelledFlights = 0;


  for (const r of rows) {
    const flights = r.totalFlights ?? 0;
    totalFlights += flights;

    if (r.delayPct != null && flights > 0) {
      delayedFlights += Math.round((r.delayPct / 100) * flights);
    }

    if (r.avgDelayMinutes != null) {
      totalDelayMin += r.avgDelayMinutes * flights;
      delayMinCount += flights;
    }

    if (r.cancellationPct != null && flights > 0) {
      cancelledFlights += Math.round(
        (r.cancellationPct / 100) * flights,
      );
    }

    const monthAtfm =
      r.delayPct != null ? (r.delayPct / 100) * flights : 0;
    if (r.delayWeatherPct != null)
      weatherMin += (r.delayWeatherPct / 100) * monthAtfm;
    if (r.delayCarrierPct != null)
      carrierMin += (r.delayCarrierPct / 100) * monthAtfm;
    if (r.delayAtcPct != null)
      atcMin += (r.delayAtcPct / 100) * monthAtfm;
    if (r.delayAirportPct != null)
      airportMin += (r.delayAirportPct / 100) * monthAtfm;
    if (monthAtfm > 0) totalAtfmMin += monthAtfm;
  }

  const delayPct =
    totalFlights > 0 ? (delayedFlights / totalFlights) * 100 : null;
  const avgDelay = delayMinCount > 0 ? totalDelayMin / delayMinCount : null;
  const cancellationPct =
    totalFlights > 0 ? (cancelledFlights / totalFlights) * 100 : null;

  return {
    totalFlights,
    delayPct,
    avgDelayMinutes: avgDelay,
    cancellationPct:
      cancellationPct && cancellationPct > 0 ? cancellationPct : null,
    delayWeatherPct:
      totalAtfmMin > 0 ? (weatherMin / totalAtfmMin) * 100 : null,
    delayCarrierPct:
      totalAtfmMin > 0 ? (carrierMin / totalAtfmMin) * 100 : null,
    delayAtcPct: totalAtfmMin > 0 ? (atcMin / totalAtfmMin) * 100 : null,
    delayAirportPct:
      totalAtfmMin > 0 ? (airportMin / totalAtfmMin) * 100 : null,
    periodLabel:
      rows.length > 1
        ? `${rows[rows.length - 1].periodYear}/${String(rows[rows.length - 1].periodMonth).padStart(2, "0")}–${rows[0].periodYear}/${String(rows[0].periodMonth).padStart(2, "0")}`
        : rows[0]?.periodYear
          ? `${rows[0].periodYear}/${String(rows[0].periodMonth).padStart(2, "0")}`
          : "",
  };
}

export function computeOpsTrend(allOps: OpsRow[]) {
  if (allOps.length < 24) return null;
  const recent = aggregateOps(allOps.slice(0, 12));
  const prior = aggregateOps(allOps.slice(12, 24));
  if (recent.delayPct == null || prior.delayPct == null) return null;
  return {
    delayChange: recent.delayPct - prior.delayPct,
    avgDelayChange:
      recent.avgDelayMinutes != null && prior.avgDelayMinutes != null
        ? recent.avgDelayMinutes - prior.avgDelayMinutes
        : null,
  };
}
