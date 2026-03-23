import { Airport } from "~/utils/types";
import { HeaderText } from "../header-text";
import { useSingleAirport } from "~/hooks/use-single-airport";
import { Stat } from "../stat";
import { scoreColor } from "~/utils/scoring";
import { fmt } from "~/utils/format";
import { ReviewCard } from "./review.card";
import { SentimentTimeline } from "./timeline";
import { SentimentBar } from "./bar";

export const Sentiment = ({ airport }: { airport: Airport }) => {
  const { latestSentiment, googleCount, skytraxCount } = useSingleAirport({
    airport,
  });

  const sentimentMetrics = [
    {
      l: "Positive",
      v: latestSentiment?.positivePct,
      c:
        latestSentiment?.positivePct &&
        parseFloat(latestSentiment?.positivePct) >= 50
          ? "text-green-500"
          : latestSentiment?.positivePct &&
              parseFloat(latestSentiment?.positivePct) >= 30
            ? "text-yellow-500"
            : "text-red-500",
    },
    {
      l: "Neutral",
      v: latestSentiment?.neutralPct,
      c: "text-zinc-400",
    },
    {
      l: "Negative",
      v: latestSentiment?.negativePct,
      c: "text-red-500",
    },
  ];

  return (
    <section className="flex flex-col gap-5 bg-[#0d0d0f] -mx-16 px-16 py-8">
      <HeaderText>What People Think</HeaderText>

      {/* Commentary from latest snapshot notes */}
      {latestSentiment?.notes && (
        <p className="font-mono text-[14px] text-zinc-400 italic leading-relaxed max-w-2xl">
          {latestSentiment.notes}
        </p>
      )}

      {latestSentiment ? (
        <>
          <div className="flex gap-8">
            <Stat
              value={
                latestSentiment.avgRating
                  ? parseFloat(latestSentiment.avgRating).toFixed(1)
                  : "—"
              }
              label="Avg Rating / 10"
              color={scoreColor(
                latestSentiment.avgRating
                  ? parseFloat(latestSentiment.avgRating) * 10
                  : null,
              )}
            />
            <div className="flex-1 flex flex-col gap-1">
              <span
                className={`font-grotesk text-[42px] font-bold text-zinc-100 tabular-nums`}
              >
                {googleCount + skytraxCount > 0
                  ? fmt(googleCount + skytraxCount)
                  : latestSentiment.reviewCount
                    ? fmt(latestSentiment.reviewCount)
                    : "—"}
              </span>
              <div className="flex items-center gap-2">
                <span className="font-mono text-[11px] text-zinc-500 tracking-wider uppercase">
                  Reviews
                </span>
                {googleCount > 0 && (
                  <span className="font-mono text-[10px] text-zinc-500 border border-zinc-700 px-1.5 py-0.5 rounded">
                    Google {googleCount}
                  </span>
                )}
                {skytraxCount > 0 && (
                  <span className="font-mono text-[10px] text-zinc-500 border border-zinc-700 px-1.5 py-0.5 rounded">
                    Skytrax {skytraxCount}
                  </span>
                )}
              </div>
            </div>
            <Stat
              value={
                latestSentiment.positivePct
                  ? `${parseFloat(latestSentiment.positivePct).toFixed(0)}%`
                  : "—"
              }
              label="Positive"
              color={
                latestSentiment.positivePct &&
                parseFloat(latestSentiment.positivePct) < 30
                  ? "text-red-500"
                  : "text-zinc-100"
              }
            />
          </div>

          {/* Skytrax stars + Google Maps rating on same row */}
          <div className="flex gap-6 items-center flex-wrap">
            {latestSentiment.skytraxStars && (
              <div className="flex gap-3 items-center">
                <span className="font-grotesk text-[10px] font-bold text-zinc-500 tracking-wider">
                  SKYTRAX STARS:
                </span>
                <span className="font-mono text-sm font-bold text-yellow-400">
                  {"★".repeat(latestSentiment.skytraxStars)}
                  {"☆".repeat(5 - latestSentiment.skytraxStars)}
                </span>
              </div>
            )}
            {airport.googleAgg.rating != null &&
              airport.googleAgg.count > 0 && (
                <div className="flex gap-2 items-center">
                  <span className="font-mono text-sm font-bold text-yellow-400">
                    ★ {(airport.googleAgg.rating / 2).toFixed(1)}
                  </span>
                  <span className="font-mono text-[10px] text-zinc-500">
                    on Google Maps ({fmt(airport.googleAgg.count)} reviews)
                  </span>
                </div>
              )}
          </div>

          {airport.recentReviews.length > 0 && (
            <div
              className="flex gap-3 overflow-x-auto pb-2"
              style={{ scrollbarWidth: "thin" }}
            >
              {airport.recentReviews.map((r, i) => (
                <ReviewCard key={i} review={r} />
              ))}
            </div>
          )}

          <SentimentTimeline snapshots={airport.sentimentSnapshots} />

          <div className="flex gap-6">
            {sentimentMetrics.map((s) => (
              <div key={s.l} className="flex gap-2 items-center">
                <span className="font-grotesk text-[10px] font-bold text-zinc-500 tracking-wider uppercase">
                  {s.l}
                </span>
                <span className={`font-mono text-xs font-bold ${s.c}`}>
                  {s.v ? `${parseFloat(s.v).toFixed(0)}%` : "—"}
                </span>
              </div>
            ))}
          </div>

          <div className="flex gap-5">
            <div className="flex-1 flex flex-col gap-2">
              <SentimentBar
                label="Queuing"
                score={latestSentiment.scoreQueuing}
              />
              <SentimentBar
                label="Cleanliness"
                score={latestSentiment.scoreCleanliness}
              />
              <SentimentBar label="Staff" score={latestSentiment.scoreStaff} />
              <SentimentBar
                label="Food & Bev"
                score={latestSentiment.scoreFoodBev}
              />
            </div>
            <div className="flex-1 flex flex-col gap-2">
              <SentimentBar label="Wifi" score={latestSentiment.scoreWifi} />
              <SentimentBar
                label="Wayfinding"
                score={latestSentiment.scoreWayfinding}
              />
              <SentimentBar
                label="Transport"
                score={latestSentiment.scoreTransport}
              />
              <SentimentBar
                label="Shopping"
                score={latestSentiment.scoreShopping}
              />
            </div>
          </div>
        </>
      ) : (
        <p className="font-mono text-xs text-zinc-600 italic">
          No sentiment data yet. The silence is deafening.
        </p>
      )}
    </section>
  );
};
