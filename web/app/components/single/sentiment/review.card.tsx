import type { Review } from "~/utils/types";

export function ReviewCard({
  review,
}: {
  review: Review;
}) {
  const rating = review.overallRating ?? 0;
  const borderColor =
    rating >= 7
      ? "border-l-green-500"
      : rating < 5
        ? "border-l-red-500"
        : "border-l-yellow-500";
  const stars = Math.round(rating / 2);
  const text = review.reviewText ?? "";
  const truncated =
    text.length > 150 ? text.slice(0, 150).trim() + "..." : text;
  const dateStr = review.reviewDate
    ? new Date(review.reviewDate).toLocaleDateString("en-US", {
        year: "numeric",
        month: "short",
      })
    : null;

  return (
    <div
      className={`shrink-0 w-[280px] border-l-2 ${borderColor} bg-[#111113] px-4 py-3 flex flex-col gap-2`}
    >
      <div className="flex items-center justify-between">
        <span className="font-mono text-[11px] text-yellow-400">
          {"★".repeat(stars)}
          {"☆".repeat(5 - stars)}
        </span>
        <span className="font-mono text-[9px] text-zinc-600 uppercase">
          {review.source}
        </span>
      </div>
      {dateStr && (
        <span className="font-mono text-[9px] text-zinc-600">{dateStr}</span>
      )}
      <p className="font-mono text-[10px] text-zinc-400 leading-relaxed">
        {truncated}
      </p>
    </div>
  );
}
