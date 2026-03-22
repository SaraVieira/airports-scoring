import { t as Route } from "./airport._iata-V9-nTsQx.js";
import { useMemo, useState } from "react";
import { Fragment, jsx, jsxs } from "react/jsx-runtime";
//#region app/routes/airport.$iata.tsx?tsr-split=component
function fmt(n) {
	if (n == null) return "—";
	const num = typeof n === "string" ? parseFloat(n) : n;
	if (isNaN(num)) return "—";
	return num.toLocaleString("en-US");
}
function fmtM(n) {
	if (n == null) return "—";
	const num = typeof n === "string" ? parseFloat(n) : n;
	if (isNaN(num)) return "—";
	if (num >= 1e6) return `${(num / 1e6).toFixed(1)}M`;
	if (num >= 1e3) return `${(num / 1e3).toFixed(0)}K`;
	return fmt(num);
}
function scoreColor(score) {
	if (score == null) return "text-zinc-600";
	if (score >= 70) return "text-green-500";
	if (score >= 40) return "text-yellow-500";
	return "text-red-500";
}
function scoreBg(score) {
	if (score == null) return "bg-zinc-600";
	if (score >= 70) return "bg-green-500";
	if (score >= 40) return "bg-yellow-500";
	return "bg-red-500";
}
function scoreVerdict(score) {
	if (score == null) return "No data";
	if (score >= 90) return "Suspiciously good";
	if (score >= 70) return "Actually decent";
	if (score >= 50) return "Passable";
	if (score >= 30) return "Painful";
	return "Dire";
}
function totalVerdict(score) {
	if (score == null) return "Unscored";
	if (score >= 81) return "Fine. We'll allow it.";
	if (score >= 61) return "Surprisingly not awful";
	if (score >= 41) return "Could be worse (but not by much)";
	if (score >= 21) return "A masterclass in mediocrity";
	return "Impressively terrible";
}
function totalCommentary(score) {
	if (!score) return "";
	if (score.commentary) return score.commentary;
	const infra = parseFloat(score.scoreInfrastructure ?? "0");
	const ops = parseFloat(score.scoreOperational ?? "0");
	const sent = parseFloat(score.scoreSentiment ?? "0");
	const conn = parseFloat(score.scoreConnectivity ?? "0");
	const parts = [];
	if (conn >= 70 && ops < 50) parts.push("Strong connectivity can't save poor operations.");
	if (infra < 40) parts.push("Infrastructure is the weak link.");
	if (sent < 40) parts.push("Passengers have noticed — and they're not happy about it.");
	const vel = parseFloat(score.scoreSentimentVelocity ?? "50");
	if (vel > 60) parts.push("At least the trend is improving.");
	else if (vel < 40) parts.push("And it's getting worse.");
	else parts.push("The trajectory is flat — no improvement in sight.");
	return parts.join(" ") || "The data speaks for itself.";
}
function paxSnark(latest, capacity) {
	if (!latest || !capacity) return "";
	const pct = Math.round(latest / capacity * 100);
	if (pct > 100) return `Running at ${pct}% capacity. The airport is literally bursting.`;
	if (pct > 85) return `Running at ${pct}% capacity. Efficiently full without feeling cramped. Show-offs.`;
	if (pct > 60) return `Running at ${pct}% capacity. The remaining ${100 - pct}% is probably the baggage claim area everyone avoids.`;
	return `Running at ${pct}% capacity. Plenty of room — and plenty of reasons people aren't coming.`;
}
function aggregateOps(rows) {
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
	let mishandledSum = 0;
	let mishandledCount = 0;
	for (const r of rows) {
		const flights = r.totalFlights ?? 0;
		totalFlights += flights;
		if (r.delayPct != null && flights > 0) delayedFlights += Math.round(parseFloat(r.delayPct) / 100 * flights);
		if (r.avgDelayMinutes != null) {
			totalDelayMin += parseFloat(r.avgDelayMinutes) * flights;
			delayMinCount += flights;
		}
		if (r.cancellationPct != null && flights > 0) cancelledFlights += Math.round(parseFloat(r.cancellationPct) / 100 * flights);
		const monthAtfm = r.delayPct != null ? parseFloat(r.delayPct) / 100 * flights : 0;
		if (r.delayWeatherPct != null) weatherMin += parseFloat(r.delayWeatherPct) / 100 * monthAtfm;
		if (r.delayCarrierPct != null) carrierMin += parseFloat(r.delayCarrierPct) / 100 * monthAtfm;
		if (r.delayAtcPct != null) atcMin += parseFloat(r.delayAtcPct) / 100 * monthAtfm;
		if (r.delayAirportPct != null) airportMin += parseFloat(r.delayAirportPct) / 100 * monthAtfm;
		if (monthAtfm > 0) totalAtfmMin += monthAtfm;
		if (r.mishandledBagsPer1k != null) {
			mishandledSum += parseFloat(r.mishandledBagsPer1k);
			mishandledCount++;
		}
	}
	const delayPct = totalFlights > 0 ? delayedFlights / totalFlights * 100 : null;
	const avgDelay = delayMinCount > 0 ? totalDelayMin / delayMinCount : null;
	const cancellationPct = totalFlights > 0 ? cancelledFlights / totalFlights * 100 : null;
	return {
		totalFlights,
		delayPct,
		avgDelayMinutes: avgDelay,
		cancellationPct: cancellationPct && cancellationPct > 0 ? cancellationPct : null,
		delayWeatherPct: totalAtfmMin > 0 ? weatherMin / totalAtfmMin * 100 : null,
		delayCarrierPct: totalAtfmMin > 0 ? carrierMin / totalAtfmMin * 100 : null,
		delayAtcPct: totalAtfmMin > 0 ? atcMin / totalAtfmMin * 100 : null,
		delayAirportPct: totalAtfmMin > 0 ? airportMin / totalAtfmMin * 100 : null,
		mishandledBagsPer1k: mishandledCount > 0 ? mishandledSum / mishandledCount : null,
		periodLabel: rows.length > 1 ? `${rows[rows.length - 1].periodYear}/${String(rows[rows.length - 1].periodMonth).padStart(2, "0")}–${rows[0].periodYear}/${String(rows[0].periodMonth).padStart(2, "0")}` : rows[0]?.periodYear ? `${rows[0].periodYear}/${String(rows[0].periodMonth).padStart(2, "0")}` : ""
	};
}
function delaySnark(delayPct) {
	if (delayPct == null) return "";
	const pct = parseFloat(String(delayPct));
	if (pct > 40) return "Nearly half of flights delayed. At this point, 'on time' is the exception.";
	if (pct > 25) return "Nearly a third of flights delayed. Pack a book. Maybe two.";
	if (pct > 15) return "One in five flights delayed. Not great, not apocalyptic.";
	if (pct > 8) return `${pct.toFixed(0)}% of flights delayed. Under ten percent. We checked twice.`;
	return "Delays are genuinely rare here. We're suspicious.";
}
function computeOpsTrend(allOps) {
	if (allOps.length < 24) return null;
	const recent = aggregateOps(allOps.slice(0, 12));
	const prior = aggregateOps(allOps.slice(12, 24));
	if (recent.delayPct == null || prior.delayPct == null) return null;
	return {
		delayChange: recent.delayPct - prior.delayPct,
		avgDelayChange: recent.avgDelayMinutes != null && prior.avgDelayMinutes != null ? recent.avgDelayMinutes - prior.avgDelayMinutes : null
	};
}
function ScoreBar({ label, score, weight }) {
	const num = score ? parseFloat(score) : null;
	const width = num != null ? `${Math.min(num, 100)}%` : "0%";
	return /* @__PURE__ */ jsxs("div", {
		className: "flex items-center gap-2 w-full",
		children: [
			/* @__PURE__ */ jsx("span", {
				className: "font-grotesk text-[11px] font-bold text-zinc-500 tracking-wider w-36 shrink-0 uppercase",
				children: label
			}),
			/* @__PURE__ */ jsx("span", {
				className: "font-mono text-[10px] text-zinc-600 w-8 shrink-0 tabular-nums",
				children: weight
			}),
			/* @__PURE__ */ jsx("div", {
				className: "flex-1 h-2 bg-zinc-900 relative",
				children: /* @__PURE__ */ jsx("div", {
					className: `h-2 ${scoreBg(num)} absolute left-0 top-0 transition-all duration-500`,
					style: { width }
				})
			}),
			/* @__PURE__ */ jsx("span", {
				className: `font-mono text-xs font-bold w-7 shrink-0 tabular-nums ${scoreColor(num)}`,
				children: num != null ? Math.round(num) : "—"
			}),
			/* @__PURE__ */ jsx("span", {
				className: `font-mono text-[11px] italic w-[120px] shrink-0 ${scoreColor(num)}`,
				children: scoreVerdict(num)
			})
		]
	});
}
function SentimentBar({ label, score }) {
	const num = score != null ? parseFloat(String(score)) : null;
	const width = num != null ? `${num / 10 * 100}%` : "0%";
	return /* @__PURE__ */ jsxs("div", {
		className: "flex items-center gap-2 w-full",
		children: [
			/* @__PURE__ */ jsx("span", {
				className: "font-grotesk text-[10px] font-bold text-zinc-500 tracking-wider w-24 shrink-0 uppercase",
				children: label
			}),
			/* @__PURE__ */ jsx("div", {
				className: "flex-1 h-1.5 bg-zinc-900 relative",
				children: /* @__PURE__ */ jsx("div", {
					className: `h-1.5 absolute left-0 top-0 ${num != null && num >= 6 ? "bg-green-500" : num != null && num >= 4 ? "bg-yellow-500" : "bg-red-500"}`,
					style: { width }
				})
			}),
			/* @__PURE__ */ jsx("span", {
				className: `font-mono text-[11px] font-bold w-7 shrink-0 tabular-nums ${num != null && num >= 6 ? "text-green-500" : num != null && num >= 4 ? "text-yellow-500" : "text-red-500"}`,
				children: num != null ? num.toFixed(1) : "—"
			})
		]
	});
}
function Divider() {
	return /* @__PURE__ */ jsx("div", { className: "w-full h-px bg-zinc-800" });
}
function ExhibitHeader({ children }) {
	return /* @__PURE__ */ jsx("h3", {
		className: "font-grotesk text-[13px] font-bold text-yellow-400 tracking-[2px] uppercase",
		children
	});
}
function Stat({ value, label, color = "text-zinc-100", size = "text-[42px]" }) {
	return /* @__PURE__ */ jsxs("div", {
		className: "flex-1 flex flex-col gap-1",
		children: [/* @__PURE__ */ jsx("span", {
			className: `font-grotesk ${size} font-bold ${color} tabular-nums`,
			children: value
		}), /* @__PURE__ */ jsx("span", {
			className: "font-mono text-[11px] text-zinc-500 tracking-wider uppercase",
			children: label
		})]
	});
}
function TrendIndicator({ value, suffix = "", invert = false }) {
	if (value == null) return null;
	const color = (invert ? value < 0 : value > 0) ? "text-green-500" : "text-red-500";
	const arrow = value > 0 ? "+" : "";
	return /* @__PURE__ */ jsxs("span", {
		className: `font-mono text-[11px] font-bold ${color}`,
		children: [
			arrow,
			value.toFixed(1),
			suffix,
			" vs prior year"
		]
	});
}
function PaxSparkline({ data }) {
	if (data.length === 0) return null;
	const maxPax = Math.max(...data.map((d) => d.pax ?? 0));
	if (maxPax === 0) return null;
	const covidYear = data.find((d) => d.year >= 2020 && d.year <= 2021 && d.pax != null && d.pax < maxPax * .5);
	return /* @__PURE__ */ jsx("div", {
		className: "flex items-end gap-[3px] h-16",
		children: data.map((d) => {
			const h = d.pax ? Math.max(d.pax / maxPax * 100, 3) : 3;
			const isCovid = d.year === covidYear?.year;
			const isLatest = d === data[0];
			return /* @__PURE__ */ jsxs("div", {
				className: "flex flex-col items-center gap-1 flex-1",
				children: [/* @__PURE__ */ jsx("div", {
					className: "w-full flex flex-col items-center justify-end h-12",
					children: /* @__PURE__ */ jsx("div", {
						className: `w-full max-w-[24px] ${isCovid ? "bg-red-500/70" : isLatest ? "bg-yellow-400" : "bg-zinc-600"} transition-all`,
						style: { height: `${h}%` }
					})
				}), /* @__PURE__ */ jsx("span", {
					className: `font-mono text-[9px] tabular-nums ${isLatest ? "text-zinc-300" : isCovid ? "text-red-500" : "text-zinc-600"}`,
					children: String(d.year).slice(2)
				})]
			}, d.year);
		})
	});
}
function SentimentTimeline({ snapshots }) {
	const byYear = /* @__PURE__ */ new Map();
	for (const s of snapshots) {
		if (s.avgRating == null) continue;
		const year = s.snapshotYear;
		const entry = byYear.get(year) ?? {
			ratings: [],
			reviews: 0
		};
		entry.ratings.push(parseFloat(String(s.avgRating)));
		entry.reviews += s.reviewCount ?? 0;
		byYear.set(year, entry);
	}
	const years = Array.from(byYear.entries()).map(([year, data]) => ({
		year,
		avg: data.ratings.reduce((a, b) => a + b, 0) / data.ratings.length,
		reviews: data.reviews
	})).sort((a, b) => a.year - b.year);
	if (years.length < 2) return null;
	const maxRating = 5;
	const first = years[0];
	const last = years[years.length - 1];
	const delta = last.avg - first.avg;
	return /* @__PURE__ */ jsxs("div", {
		className: "flex flex-col gap-3",
		children: [
			/* @__PURE__ */ jsxs("div", {
				className: "flex items-center justify-between",
				children: [/* @__PURE__ */ jsxs("span", {
					className: "font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase",
					children: [
						"Sentiment Trajectory (",
						first.year,
						"–",
						last.year,
						")"
					]
				}), /* @__PURE__ */ jsxs("span", {
					className: `font-mono text-[11px] font-bold ${delta > .2 ? "text-green-500" : delta < -.2 ? "text-red-500" : "text-zinc-500"}`,
					children: [
						delta > 0 ? "+" : "",
						delta.toFixed(2),
						" over ",
						years.length,
						" years"
					]
				})]
			}),
			/* @__PURE__ */ jsx("div", {
				className: "flex items-end gap-[2px] h-20",
				children: years.map((y) => {
					const h = Math.max(y.avg / maxRating * 100, 8);
					const color = y.avg >= 3.5 ? "bg-green-500/70" : y.avg >= 2.5 ? "bg-yellow-500/70" : "bg-red-500/70";
					return /* @__PURE__ */ jsxs("div", {
						className: "flex flex-col items-center gap-1 flex-1",
						title: `${y.year}: ${y.avg.toFixed(2)} (${y.reviews} reviews)`,
						children: [
							/* @__PURE__ */ jsx("span", {
								className: "font-mono text-[9px] text-zinc-600 tabular-nums",
								children: y.avg.toFixed(1)
							}),
							/* @__PURE__ */ jsx("div", {
								className: "w-full flex justify-center h-14",
								children: /* @__PURE__ */ jsx("div", {
									className: `w-full max-w-[28px] ${color}`,
									style: {
										height: `${h}%`,
										alignSelf: "flex-end"
									}
								})
							}),
							/* @__PURE__ */ jsx("span", {
								className: "font-mono text-[9px] text-zinc-600 tabular-nums",
								children: String(y.year).slice(2)
							})
						]
					}, y.year);
				})
			}),
			/* @__PURE__ */ jsxs("div", {
				className: "flex justify-between",
				children: [/* @__PURE__ */ jsxs("span", {
					className: "font-mono text-[10px] text-zinc-600",
					children: [
						"Then: ",
						first.avg.toFixed(1),
						"/5 (",
						first.reviews,
						" reviews)"
					]
				}), /* @__PURE__ */ jsxs("span", {
					className: `font-mono text-[10px] font-bold ${last.avg > first.avg ? "text-green-500" : "text-red-500"}`,
					children: [
						"Now: ",
						last.avg.toFixed(1),
						"/5 (",
						last.reviews,
						" reviews)"
					]
				})]
			})
		]
	});
}
function AirportDetail() {
	const airport = Route.useLoaderData();
	const score = airport.scores[0];
	const totalNum = score?.scoreTotal ? parseFloat(score.scoreTotal) : null;
	const latestPax = airport.paxYearly[0];
	const prevPax = airport.paxYearly[1];
	const recentOps = airport.operationalStats.slice(0, 12);
	const opsAgg = recentOps.length > 0 ? aggregateOps(recentOps) : null;
	const opsTrend = computeOpsTrend(airport.operationalStats);
	const latestSentiment = useMemo(() => {
		const snaps = airport.sentimentSnapshots;
		if (snaps.length === 0) return null;
		let totalRating = 0, ratingCount = 0;
		let totalReviews = 0;
		let totalPositive = 0, totalNegative = 0, totalNeutral = 0, pctCount = 0;
		let queueSum = 0, queueN = 0;
		let cleanSum = 0, cleanN = 0;
		let staffSum = 0, staffN = 0;
		let foodSum = 0, foodN = 0;
		let wifiSum = 0, wifiN = 0;
		let waySum = 0, wayN = 0;
		let transSum = 0, transN = 0;
		let shopSum = 0, shopN = 0;
		let skytraxStars = snaps[0].skytraxStars;
		for (const s of snaps) {
			if (s.avgRating != null) {
				totalRating += parseFloat(String(s.avgRating));
				ratingCount++;
			}
			if (s.reviewCount != null) totalReviews += s.reviewCount;
			if (s.positivePct != null) {
				totalPositive += parseFloat(String(s.positivePct));
				pctCount++;
			}
			if (s.negativePct != null) totalNegative += parseFloat(String(s.negativePct));
			if (s.neutralPct != null) totalNeutral += parseFloat(String(s.neutralPct));
			if (s.scoreQueuing != null) {
				queueSum += parseFloat(String(s.scoreQueuing));
				queueN++;
			}
			if (s.scoreCleanliness != null) {
				cleanSum += parseFloat(String(s.scoreCleanliness));
				cleanN++;
			}
			if (s.scoreStaff != null) {
				staffSum += parseFloat(String(s.scoreStaff));
				staffN++;
			}
			if (s.scoreFoodBev != null) {
				foodSum += parseFloat(String(s.scoreFoodBev));
				foodN++;
			}
			if (s.scoreWifi != null) {
				wifiSum += parseFloat(String(s.scoreWifi));
				wifiN++;
			}
			if (s.scoreWayfinding != null) {
				waySum += parseFloat(String(s.scoreWayfinding));
				wayN++;
			}
			if (s.scoreTransport != null) {
				transSum += parseFloat(String(s.scoreTransport));
				transN++;
			}
			if (s.scoreShopping != null) {
				shopSum += parseFloat(String(s.scoreShopping));
				shopN++;
			}
			if (s.skytraxStars != null) skytraxStars = s.skytraxStars;
		}
		return {
			avgRating: ratingCount > 0 ? String((totalRating / ratingCount).toFixed(2)) : null,
			reviewCount: totalReviews,
			positivePct: pctCount > 0 ? String((totalPositive / pctCount).toFixed(2)) : null,
			negativePct: pctCount > 0 ? String((totalNegative / pctCount).toFixed(2)) : null,
			neutralPct: pctCount > 0 ? String((totalNeutral / pctCount).toFixed(2)) : null,
			scoreQueuing: queueN > 0 ? String((queueSum / queueN).toFixed(2)) : null,
			scoreCleanliness: cleanN > 0 ? String((cleanSum / cleanN).toFixed(2)) : null,
			scoreStaff: staffN > 0 ? String((staffSum / staffN).toFixed(2)) : null,
			scoreFoodBev: foodN > 0 ? String((foodSum / foodN).toFixed(2)) : null,
			scoreWifi: wifiN > 0 ? String((wifiSum / wifiN).toFixed(2)) : null,
			scoreWayfinding: wayN > 0 ? String((waySum / wayN).toFixed(2)) : null,
			scoreTransport: transN > 0 ? String((transSum / transN).toFixed(2)) : null,
			scoreShopping: shopN > 0 ? String((shopSum / shopN).toFixed(2)) : null,
			skytraxStars,
			snapshotCount: snaps.length
		};
	}, [airport.sentimentSnapshots]);
	const wiki = airport.wikipediaSnapshots[0];
	const routesWithFlights = airport.routesOut.filter((r) => r.flightsPerMonth != null && r.flightsPerMonth > 0);
	const yoyGrowth = latestPax?.totalPax && prevPax?.totalPax ? (latestPax.totalPax - prevPax.totalPax) / prevPax.totalPax * 100 : null;
	const capacityNum = airport.annualCapacityM ? parseFloat(airport.annualCapacityM) * 1e6 : null;
	const latestPaxNum = latestPax?.totalPax ?? null;
	const paxYears = airport.paxYearly.map((p) => p.year).filter(Boolean);
	const opsYears = airport.operationalStats.map((o) => o.periodYear).filter(Boolean);
	const sentYears = airport.sentimentSnapshots.map((s) => s.snapshotYear).filter(Boolean);
	const allYears = [
		...paxYears,
		...opsYears,
		...sentYears
	];
	const dataRange = allYears.length > 0 ? `Based on data from ${Math.min(...allYears)}–${Math.max(...allYears)}` : null;
	const paxSparkData = [...airport.paxYearly].reverse().map((p) => ({
		year: p.year,
		pax: p.totalPax
	}));
	return /* @__PURE__ */ jsx("div", {
		className: "min-h-screen bg-[#0a0a0b] text-zinc-100",
		children: /* @__PURE__ */ jsxs("div", {
			className: "max-w-5xl mx-auto px-16 py-12 flex flex-col gap-12",
			children: [
				/* @__PURE__ */ jsxs("header", {
					className: "flex flex-col gap-0",
					children: [
						/* @__PURE__ */ jsx("span", {
							className: "font-grotesk text-[120px] font-bold text-white/7 leading-none tracking-[8px]",
							children: airport.iataCode
						}),
						/* @__PURE__ */ jsx("h1", {
							className: "font-grotesk text-[28px] font-bold text-zinc-100 tracking-wide",
							children: airport.name
						}),
						/* @__PURE__ */ jsxs("p", {
							className: "font-mono text-[13px] text-zinc-500 tracking-[1.5px] uppercase",
							children: [
								airport.city,
								", ",
								airport.country?.name
							]
						}),
						airport.operator && /* @__PURE__ */ jsxs("p", {
							className: "font-mono text-[11px] text-zinc-600 tracking-wider uppercase",
							children: ["Operated by ", airport.operator.name]
						}),
						/* @__PURE__ */ jsxs("div", {
							className: "flex gap-3 mt-3 flex-wrap",
							children: [
								airport.openedYear && /* @__PURE__ */ jsx(Badge, {
									label: "Opened",
									value: String(airport.openedYear),
									bright: true
								}),
								airport.icaoCode && /* @__PURE__ */ jsx(Badge, {
									label: "ICAO",
									value: airport.icaoCode
								}),
								airport.terminalCount && /* @__PURE__ */ jsx(Badge, {
									label: "Terminals",
									value: String(airport.terminalCount)
								}),
								airport.totalGates && /* @__PURE__ */ jsx(Badge, {
									label: "Gates",
									value: String(airport.totalGates)
								}),
								airport.elevationFt && /* @__PURE__ */ jsx(Badge, {
									label: "Elev",
									value: `${fmt(airport.elevationFt)} ft`
								})
							]
						}),
						airport.owner && /* @__PURE__ */ jsxs("p", {
							className: "font-mono text-[11px] text-zinc-600 mt-2 uppercase",
							children: [
								/* @__PURE__ */ jsx("span", {
									className: "font-grotesk text-[9px] font-bold text-zinc-600 tracking-wider",
									children: "Owner:"
								}),
								" ",
								airport.owner.name,
								airport.ownershipNotes ? ` — ${airport.ownershipNotes}` : ""
							]
						})
					]
				}),
				/* @__PURE__ */ jsx(Divider, {}),
				/* @__PURE__ */ jsxs("section", {
					className: "flex flex-col items-center gap-2 py-8",
					children: [
						/* @__PURE__ */ jsx("span", {
							className: "font-grotesk text-[11px] font-bold text-zinc-500 tracking-[2px] uppercase",
							children: "The Verdict"
						}),
						/* @__PURE__ */ jsx("span", {
							className: `font-grotesk text-[96px] font-bold leading-none tabular-nums ${scoreColor(totalNum)}`,
							children: totalNum != null ? Math.round(totalNum) : "?"
						}),
						/* @__PURE__ */ jsx("span", {
							className: "font-mono text-lg text-zinc-600",
							children: "/100"
						}),
						/* @__PURE__ */ jsx("span", {
							className: `font-mono text-sm italic ${scoreColor(totalNum)}`,
							children: totalVerdict(totalNum)
						}),
						/* @__PURE__ */ jsx("p", {
							className: "font-mono text-xs text-zinc-600 italic text-center max-w-2xl mt-2 leading-relaxed",
							children: totalCommentary(score)
						}),
						dataRange && /* @__PURE__ */ jsx("span", {
							className: "font-mono text-[10px] text-zinc-700 mt-1",
							children: dataRange
						})
					]
				}),
				/* @__PURE__ */ jsxs("div", {
					className: "flex flex-col gap-3 pb-6",
					children: [
						/* @__PURE__ */ jsx(ScoreBar, {
							label: "Operational",
							score: score?.scoreOperational,
							weight: "25%"
						}),
						/* @__PURE__ */ jsx(ScoreBar, {
							label: "Sentiment",
							score: score?.scoreSentiment,
							weight: "25%"
						}),
						/* @__PURE__ */ jsx(ScoreBar, {
							label: "Infrastructure",
							score: score?.scoreInfrastructure,
							weight: "15%"
						}),
						/* @__PURE__ */ jsx(ScoreBar, {
							label: "Sent. Velocity",
							score: score?.scoreSentimentVelocity,
							weight: "15%"
						}),
						/* @__PURE__ */ jsx(ScoreBar, {
							label: "Connectivity",
							score: score?.scoreConnectivity,
							weight: "10%"
						}),
						/* @__PURE__ */ jsx(ScoreBar, {
							label: "Operator",
							score: score?.scoreOperator,
							weight: "10%"
						})
					]
				}),
				/* @__PURE__ */ jsx(Divider, {}),
				/* @__PURE__ */ jsxs("section", {
					className: "flex flex-col gap-5",
					children: [
						/* @__PURE__ */ jsx(ExhibitHeader, { children: "Exhibit A — The Numbers" }),
						/* @__PURE__ */ jsxs("div", {
							className: "flex gap-8",
							children: [
								/* @__PURE__ */ jsx(Stat, {
									value: latestPax ? fmtM(latestPax.totalPax) : "—",
									label: `Passengers${latestPax ? ` (${latestPax.year})` : ""}`
								}),
								/* @__PURE__ */ jsx(Stat, {
									value: yoyGrowth != null ? `${yoyGrowth > 0 ? "+" : ""}${yoyGrowth.toFixed(1)}%` : "—",
									label: "YoY Growth",
									color: yoyGrowth != null ? yoyGrowth > 0 ? "text-green-500" : "text-red-500" : "text-zinc-600"
								}),
								/* @__PURE__ */ jsx(Stat, {
									value: capacityNum ? fmtM(capacityNum) : "—",
									label: "Annual Capacity",
									color: "text-zinc-600"
								})
							]
						}),
						/* @__PURE__ */ jsx("p", {
							className: "font-mono text-xs text-zinc-600 italic leading-relaxed",
							children: paxSnark(latestPaxNum, capacityNum)
						}),
						latestPax && /* @__PURE__ */ jsxs("div", {
							className: "flex gap-8",
							children: [
								/* @__PURE__ */ jsx(Stat, {
									value: latestPax.internationalPax ? fmtM(latestPax.internationalPax) : "—",
									label: `International${latestPax.totalPax && latestPax.internationalPax ? ` (${Math.round(latestPax.internationalPax / latestPax.totalPax * 100)}%)` : ""}`,
									size: "text-[28px]"
								}),
								/* @__PURE__ */ jsx(Stat, {
									value: latestPax.domesticPax ? fmtM(latestPax.domesticPax) : "—",
									label: `Domestic${latestPax.totalPax && latestPax.domesticPax ? ` (${Math.round(latestPax.domesticPax / latestPax.totalPax * 100)}%)` : ""}`,
									size: "text-[28px]",
									color: "text-zinc-600"
								}),
								/* @__PURE__ */ jsx(Stat, {
									value: latestPax.aircraftMovements ? fmt(latestPax.aircraftMovements) : "—",
									label: "Aircraft Movements",
									size: "text-[28px]",
									color: "text-zinc-600"
								})
							]
						}),
						paxSparkData.length > 2 && /* @__PURE__ */ jsxs(Fragment, { children: [/* @__PURE__ */ jsx("span", {
							className: "font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase",
							children: "Passenger History"
						}), /* @__PURE__ */ jsx(PaxSparkline, { data: paxSparkData })] }),
						latestPaxNum && capacityNum && /* @__PURE__ */ jsxs("div", {
							className: "flex flex-col gap-1",
							children: [/* @__PURE__ */ jsxs("div", {
								className: "flex justify-between",
								children: [/* @__PURE__ */ jsx("span", {
									className: "font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase",
									children: "Capacity Utilization"
								}), /* @__PURE__ */ jsxs("span", {
									className: "font-mono text-[11px] font-bold text-zinc-400 tabular-nums",
									children: [Math.round(latestPaxNum / capacityNum * 100), "%"]
								})]
							}), /* @__PURE__ */ jsx("div", {
								className: "h-1.5 bg-zinc-900 relative",
								children: /* @__PURE__ */ jsx("div", {
									className: `h-1.5 absolute left-0 top-0 ${latestPaxNum / capacityNum > .9 ? "bg-red-500" : latestPaxNum / capacityNum > .7 ? "bg-yellow-500" : "bg-green-500"}`,
									style: { width: `${Math.min(latestPaxNum / capacityNum * 100, 100)}%` }
								})
							})]
						})
					]
				}),
				/* @__PURE__ */ jsx(Divider, {}),
				opsAgg && /* @__PURE__ */ jsxs("section", {
					className: "flex flex-col gap-5",
					children: [
						/* @__PURE__ */ jsx(ExhibitHeader, { children: "Exhibit B — Tardiness Report" }),
						opsAgg.periodLabel && /* @__PURE__ */ jsxs("span", {
							className: "font-mono text-[10px] text-zinc-600 tracking-wider uppercase",
							children: [
								opsAgg.periodLabel,
								" · ",
								fmt(opsAgg.totalFlights),
								" flights"
							]
						}),
						/* @__PURE__ */ jsxs("div", {
							className: "flex gap-8",
							children: [
								/* @__PURE__ */ jsx(Stat, {
									value: opsAgg.delayPct != null ? `${opsAgg.delayPct.toFixed(1)}%` : "—",
									label: "Flights Delayed",
									color: scoreColor(opsAgg.delayPct != null ? 100 - opsAgg.delayPct * 2.5 : null)
								}),
								/* @__PURE__ */ jsx(Stat, {
									value: opsAgg.avgDelayMinutes != null ? `${opsAgg.avgDelayMinutes.toFixed(1)}min` : "—",
									label: "Avg Delay",
									color: scoreColor(opsAgg.avgDelayMinutes != null ? 100 - opsAgg.avgDelayMinutes * 3 : null)
								}),
								/* @__PURE__ */ jsx(Stat, {
									value: opsAgg.cancellationPct != null ? `${opsAgg.cancellationPct.toFixed(1)}%` : "—",
									label: "Cancelled",
									color: scoreColor(opsAgg.cancellationPct != null ? 100 - opsAgg.cancellationPct * 10 : null)
								})
							]
						}),
						/* @__PURE__ */ jsx("p", {
							className: "font-mono text-xs text-zinc-600 italic leading-relaxed",
							children: delaySnark(opsAgg.delayPct)
						}),
						opsTrend && /* @__PURE__ */ jsxs("div", {
							className: "flex gap-6",
							children: [/* @__PURE__ */ jsx(TrendIndicator, {
								value: opsTrend.delayChange,
								suffix: "pp",
								invert: true
							}), opsTrend.avgDelayChange != null && /* @__PURE__ */ jsx(TrendIndicator, {
								value: opsTrend.avgDelayChange,
								suffix: "min",
								invert: true
							})]
						}),
						(opsAgg.delayWeatherPct != null || opsAgg.delayAtcPct != null || opsAgg.delayAirportPct != null) && /* @__PURE__ */ jsxs(Fragment, { children: [/* @__PURE__ */ jsx("span", {
							className: "font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase",
							children: "Delay Causes (ATFM)"
						}), /* @__PURE__ */ jsx("div", {
							className: "flex gap-4",
							children: [
								{
									label: "Weather",
									val: opsAgg.delayWeatherPct
								},
								{
									label: "Carrier",
									val: opsAgg.delayCarrierPct
								},
								{
									label: "ATC",
									val: opsAgg.delayAtcPct
								},
								{
									label: "Airport",
									val: opsAgg.delayAirportPct
								}
							].map((c) => /* @__PURE__ */ jsxs("div", {
								className: "flex-1 flex justify-between",
								children: [/* @__PURE__ */ jsx("span", {
									className: "font-mono text-[11px] text-zinc-500",
									children: c.label
								}), /* @__PURE__ */ jsx("span", {
									className: `font-mono text-[11px] font-bold ${c.val != null && c.val > 25 ? "text-red-500" : c.val != null && c.val > 15 ? "text-orange-500" : "text-zinc-400"}`,
									children: c.val != null ? `${c.val.toFixed(0)}%` : "—"
								})]
							}, c.label))
						})] }),
						opsAgg.mishandledBagsPer1k != null && /* @__PURE__ */ jsxs("div", {
							className: "flex gap-2 items-center",
							children: [/* @__PURE__ */ jsx("span", {
								className: "font-grotesk text-[10px] font-bold text-zinc-500 tracking-wider",
								children: "MISHANDLED BAGS:"
							}), /* @__PURE__ */ jsxs("span", {
								className: "font-mono text-[11px] font-bold text-orange-500",
								children: [opsAgg.mishandledBagsPer1k.toFixed(1), " per 1,000 passengers"]
							})]
						})
					]
				}),
				/* @__PURE__ */ jsx(Divider, {}),
				/* @__PURE__ */ jsxs("section", {
					className: "flex flex-col gap-5",
					children: [/* @__PURE__ */ jsx(ExhibitHeader, { children: "Exhibit C — What People Think" }), latestSentiment ? /* @__PURE__ */ jsxs(Fragment, { children: [
						/* @__PURE__ */ jsxs("div", {
							className: "flex gap-8",
							children: [
								/* @__PURE__ */ jsx(Stat, {
									value: latestSentiment.avgRating ? parseFloat(latestSentiment.avgRating).toFixed(1) : "—",
									label: "Avg Rating / 10",
									color: scoreColor(latestSentiment.avgRating ? parseFloat(latestSentiment.avgRating) * 10 : null)
								}),
								/* @__PURE__ */ jsx(Stat, {
									value: latestSentiment.reviewCount ? fmt(latestSentiment.reviewCount) : "—",
									label: "Reviews"
								}),
								/* @__PURE__ */ jsx(Stat, {
									value: latestSentiment.positivePct ? `${parseFloat(latestSentiment.positivePct).toFixed(0)}%` : "—",
									label: "Positive",
									color: latestSentiment.positivePct && parseFloat(latestSentiment.positivePct) < 30 ? "text-red-500" : "text-zinc-100"
								})
							]
						}),
						latestSentiment.skytraxStars && /* @__PURE__ */ jsxs("div", {
							className: "flex gap-6 items-center",
							children: [/* @__PURE__ */ jsx("span", {
								className: "font-grotesk text-[10px] font-bold text-zinc-500 tracking-wider",
								children: "SKYTRAX STARS:"
							}), /* @__PURE__ */ jsxs("span", {
								className: "font-mono text-sm font-bold text-yellow-400",
								children: ["★".repeat(latestSentiment.skytraxStars), "☆".repeat(5 - latestSentiment.skytraxStars)]
							})]
						}),
						/* @__PURE__ */ jsx(SentimentTimeline, { snapshots: airport.sentimentSnapshots }),
						/* @__PURE__ */ jsx("div", {
							className: "flex gap-6",
							children: [
								{
									l: "Positive",
									v: latestSentiment.positivePct,
									c: "text-green-500"
								},
								{
									l: "Neutral",
									v: latestSentiment.neutralPct,
									c: "text-zinc-400"
								},
								{
									l: "Negative",
									v: latestSentiment.negativePct,
									c: "text-red-500"
								}
							].map((s) => /* @__PURE__ */ jsxs("div", {
								className: "flex gap-2 items-center",
								children: [/* @__PURE__ */ jsx("span", {
									className: "font-grotesk text-[10px] font-bold text-zinc-500 tracking-wider uppercase",
									children: s.l
								}), /* @__PURE__ */ jsx("span", {
									className: `font-mono text-xs font-bold ${s.c}`,
									children: s.v ? `${parseFloat(s.v).toFixed(0)}%` : "—"
								})]
							}, s.l))
						}),
						/* @__PURE__ */ jsxs("div", {
							className: "flex gap-5",
							children: [/* @__PURE__ */ jsxs("div", {
								className: "flex-1 flex flex-col gap-2",
								children: [
									/* @__PURE__ */ jsx(SentimentBar, {
										label: "Queuing",
										score: latestSentiment.scoreQueuing
									}),
									/* @__PURE__ */ jsx(SentimentBar, {
										label: "Cleanliness",
										score: latestSentiment.scoreCleanliness
									}),
									/* @__PURE__ */ jsx(SentimentBar, {
										label: "Staff",
										score: latestSentiment.scoreStaff
									}),
									/* @__PURE__ */ jsx(SentimentBar, {
										label: "Food & Bev",
										score: latestSentiment.scoreFoodBev
									})
								]
							}), /* @__PURE__ */ jsxs("div", {
								className: "flex-1 flex flex-col gap-2",
								children: [
									/* @__PURE__ */ jsx(SentimentBar, {
										label: "Wifi",
										score: latestSentiment.scoreWifi
									}),
									/* @__PURE__ */ jsx(SentimentBar, {
										label: "Wayfinding",
										score: latestSentiment.scoreWayfinding
									}),
									/* @__PURE__ */ jsx(SentimentBar, {
										label: "Transport",
										score: latestSentiment.scoreTransport
									}),
									/* @__PURE__ */ jsx(SentimentBar, {
										label: "Shopping",
										score: latestSentiment.scoreShopping
									})
								]
							})]
						})
					] }) : /* @__PURE__ */ jsx("p", {
						className: "font-mono text-xs text-zinc-600 italic",
						children: "No sentiment data yet. The silence is deafening."
					})]
				}),
				/* @__PURE__ */ jsx(Divider, {}),
				/* @__PURE__ */ jsx(RouteSection, { routesWithFlights }),
				/* @__PURE__ */ jsx(Divider, {}),
				/* @__PURE__ */ jsxs("section", {
					className: "flex flex-col gap-4",
					children: [
						/* @__PURE__ */ jsx(ExhibitHeader, { children: "Exhibit E — The Runway Report" }),
						/* @__PURE__ */ jsxs("span", {
							className: "font-grotesk text-[11px] font-bold text-zinc-100 tracking-wider uppercase",
							children: [
								airport.runways.length,
								" Runway",
								airport.runways.length !== 1 ? "s" : ""
							]
						}),
						/* @__PURE__ */ jsx("div", {
							className: "flex gap-6",
							children: airport.runways.map((rw) => /* @__PURE__ */ jsxs("div", {
								className: "flex-1 flex flex-col gap-2 p-5 bg-[#111113] border border-zinc-800",
								children: [/* @__PURE__ */ jsx("span", {
									className: "font-grotesk text-lg font-bold text-zinc-100 tracking-wider",
									children: rw.leIdent && rw.heIdent ? `${rw.leIdent}/${rw.heIdent}` : rw.ident ?? `Runway ${rw.id}`
								}), /* @__PURE__ */ jsxs("span", {
									className: "font-mono text-[11px] text-zinc-500",
									children: [
										[rw.lengthFt ? `${fmt(rw.lengthFt)}ft` : null, rw.widthFt ? `${rw.widthFt}ft` : null].filter(Boolean).join(" × "),
										rw.surface ? ` · ${rw.surface}` : "",
										rw.lighted ? " · Lighted" : "",
										rw.closed ? " · CLOSED" : ""
									]
								})]
							}, rw.id))
						})
					]
				}),
				/* @__PURE__ */ jsx(Divider, {}),
				wiki && /* @__PURE__ */ jsxs("section", {
					className: "flex flex-col gap-4",
					children: [
						/* @__PURE__ */ jsx(ExhibitHeader, { children: "Exhibit F — The Backstory" }),
						/* @__PURE__ */ jsx(BackstoryTimeline, {
							airport,
							wiki
						}),
						/* @__PURE__ */ jsx("span", {
							className: "font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase mt-4",
							children: "ACI Service Quality Awards"
						}),
						wiki.aciAwards && typeof wiki.aciAwards === "object" && Object.keys(wiki.aciAwards).length > 0 ? /* @__PURE__ */ jsx("div", {
							className: "grid grid-cols-[repeat(auto-fill,minmax(140px,1fr))] gap-2 mt-1",
							children: Object.entries(wiki.aciAwards).sort(([a], [b]) => b.localeCompare(a)).map(([year, placements]) => {
								const entries = Object.entries(placements);
								const place = entries[0]?.[0] ?? "";
								const category = entries[0]?.[1] ?? "";
								return /* @__PURE__ */ jsxs("div", {
									className: "border border-zinc-800 rounded px-3 py-2 flex flex-col gap-0.5",
									children: [
										/* @__PURE__ */ jsxs("div", {
											className: "flex items-center justify-between",
											children: [/* @__PURE__ */ jsx("span", {
												className: "font-mono text-xs font-bold text-zinc-300",
												children: year
											}), /* @__PURE__ */ jsx("span", {
												className: "text-sm",
												children: place === "1st" ? "🥇" : place === "2nd" ? "🥈" : place === "3rd" ? "🥉" : "🏆"
											})]
										}),
										/* @__PURE__ */ jsx("span", {
											className: "font-mono text-[10px] text-zinc-500 uppercase tracking-wide",
											children: place
										}),
										/* @__PURE__ */ jsx("span", {
											className: "font-mono text-[10px] text-zinc-600 leading-tight",
											children: category
										})
									]
								}, year);
							})
						}) : /* @__PURE__ */ jsx("p", {
							className: "font-mono text-xs text-zinc-500 italic mt-1",
							children: "None recorded. A clean record — in the worst sense."
						})
					]
				}),
				/* @__PURE__ */ jsx(Divider, {}),
				/* @__PURE__ */ jsxs("footer", {
					className: "flex gap-6",
					children: [
						airport.wikipediaUrl && /* @__PURE__ */ jsx("a", {
							href: airport.wikipediaUrl,
							target: "_blank",
							rel: "noopener noreferrer",
							className: "font-grotesk text-[11px] font-bold text-yellow-400 tracking-wider hover:text-yellow-300 transition-colors",
							children: "WIKIPEDIA ↗"
						}),
						airport.websiteUrl && /* @__PURE__ */ jsx("a", {
							href: airport.websiteUrl,
							target: "_blank",
							rel: "noopener noreferrer",
							className: "font-grotesk text-[11px] font-bold text-yellow-400 tracking-wider hover:text-yellow-300 transition-colors",
							children: "WEBSITE ↗"
						}),
						airport.skytraxUrl && /* @__PURE__ */ jsx("a", {
							href: airport.skytraxUrl,
							target: "_blank",
							rel: "noopener noreferrer",
							className: "font-grotesk text-[11px] font-bold text-yellow-400 tracking-wider hover:text-yellow-300 transition-colors",
							children: "SKYTRAX ↗"
						})
					]
				})
			]
		})
	});
}
function BackstoryTimeline({ airport, wiki }) {
	const events = [];
	if (airport.openedYear) events.push({
		year: airport.openedYear,
		label: "Opened",
		color: "text-green-500"
	});
	if (airport.lastMajorReno) events.push({
		year: airport.lastMajorReno,
		label: "Major Renovation",
		detail: wiki.renovationNotes ?? void 0,
		color: "text-yellow-400"
	});
	if (wiki.skytraxHistory && typeof wiki.skytraxHistory === "object") for (const [year, stars] of Object.entries(wiki.skytraxHistory)) events.push({
		year: parseInt(year),
		label: `${stars}-Star Skytrax Rating`,
		color: "text-yellow-400"
	});
	events.sort((a, b) => a.year - b.year);
	if (events.length === 0 && !wiki.terminalNames?.length && !wiki.renovationNotes) return null;
	return /* @__PURE__ */ jsxs("div", {
		className: "flex flex-col gap-0",
		children: [
			wiki.terminalNames && wiki.terminalNames.length > 0 && /* @__PURE__ */ jsxs("div", {
				className: "flex gap-2 items-center mb-4",
				children: [/* @__PURE__ */ jsx("span", {
					className: "font-grotesk text-[10px] font-bold text-zinc-500 tracking-wider",
					children: "TERMINALS:"
				}), /* @__PURE__ */ jsx("span", {
					className: "font-mono text-xs text-zinc-400",
					children: wiki.terminalNames.join(" · ")
				})]
			}),
			events.length > 0 && /* @__PURE__ */ jsx("div", {
				className: "flex flex-col",
				children: events.map((ev, i) => /* @__PURE__ */ jsxs("div", {
					className: "flex gap-4 items-start",
					children: [/* @__PURE__ */ jsxs("div", {
						className: "flex flex-col items-center",
						children: [/* @__PURE__ */ jsx("span", {
							className: "font-mono text-xs font-bold text-zinc-400 tabular-nums w-12 shrink-0",
							children: ev.year
						}), i < events.length - 1 && /* @__PURE__ */ jsx("div", { className: "w-px h-6 bg-zinc-800 mt-1" })]
					}), /* @__PURE__ */ jsxs("div", {
						className: "flex flex-col gap-0.5 pb-4",
						children: [/* @__PURE__ */ jsx("span", {
							className: `font-grotesk text-[11px] font-bold ${ev.color} tracking-wider uppercase`,
							children: ev.label
						}), ev.detail && /* @__PURE__ */ jsx("span", {
							className: "font-mono text-[10px] text-zinc-600 leading-relaxed",
							children: ev.detail
						})]
					})]
				}, `${ev.year}-${i}`))
			}),
			wiki.renovationNotes && !airport.lastMajorReno && /* @__PURE__ */ jsxs(Fragment, { children: [/* @__PURE__ */ jsx("span", {
				className: "font-grotesk text-[10px] font-bold text-zinc-600 tracking-[1.5px] uppercase",
				children: "Renovation Notes"
			}), /* @__PURE__ */ jsx("p", {
				className: "font-mono text-xs text-zinc-500 leading-relaxed",
				children: wiki.renovationNotes
			})] })
		]
	});
}
function routeDisplayName(r) {
	return r.destinationAirport?.name ?? r.destination?.name ?? r.destinationIata ?? r.destinationIcao ?? "Unknown";
}
function routeIata(r) {
	return r.destinationAirport?.iata ?? r.destinationIata ?? null;
}
function routeCountry(r) {
	return r.destinationAirport?.country ?? "Unknown";
}
function routeRegion(r) {
	const country = routeCountry(r);
	if ([
		"AT",
		"BE",
		"BG",
		"HR",
		"CY",
		"CZ",
		"DK",
		"EE",
		"FI",
		"FR",
		"DE",
		"GR",
		"HU",
		"IE",
		"IT",
		"LV",
		"LT",
		"LU",
		"MT",
		"NL",
		"PL",
		"PT",
		"RO",
		"SK",
		"SI",
		"ES",
		"SE",
		"GB",
		"NO",
		"CH",
		"IS",
		"AL",
		"BA",
		"ME",
		"MK",
		"RS",
		"XK",
		"UA",
		"MD",
		"BY"
	].includes(country)) return "Europe";
	if ([
		"DZ",
		"AO",
		"BJ",
		"BW",
		"BF",
		"BI",
		"CV",
		"CM",
		"CF",
		"TD",
		"KM",
		"CD",
		"CG",
		"CI",
		"DJ",
		"EG",
		"GQ",
		"ER",
		"SZ",
		"ET",
		"GA",
		"GM",
		"GH",
		"GN",
		"GW",
		"KE",
		"LS",
		"LR",
		"LY",
		"MG",
		"MW",
		"ML",
		"MR",
		"MU",
		"MA",
		"MZ",
		"NA",
		"NE",
		"NG",
		"RW",
		"ST",
		"SN",
		"SC",
		"SL",
		"SO",
		"ZA",
		"SS",
		"SD",
		"TZ",
		"TG",
		"TN",
		"UG",
		"ZM",
		"ZW"
	].includes(country)) return "Africa";
	if ([
		"AE",
		"BH",
		"IL",
		"IQ",
		"IR",
		"JO",
		"KW",
		"LB",
		"OM",
		"PS",
		"QA",
		"SA",
		"SY",
		"TR",
		"YE"
	].includes(country)) return "Middle East";
	if ([
		"AF",
		"AM",
		"AZ",
		"BD",
		"BT",
		"BN",
		"KH",
		"CN",
		"GE",
		"IN",
		"ID",
		"JP",
		"KZ",
		"KG",
		"LA",
		"MY",
		"MV",
		"MN",
		"MM",
		"NP",
		"KP",
		"PK",
		"PH",
		"RU",
		"SG",
		"KR",
		"LK",
		"TW",
		"TJ",
		"TH",
		"TL",
		"TM",
		"UZ",
		"VN"
	].includes(country)) return "Asia";
	if ([
		"US",
		"CA",
		"MX",
		"BR",
		"AR",
		"CL",
		"CO",
		"PE",
		"VE",
		"EC",
		"BO",
		"PY",
		"UY",
		"GY",
		"SR",
		"CR",
		"PA",
		"CU",
		"DO",
		"HT",
		"JM",
		"TT",
		"BS",
		"BB",
		"GT",
		"HN",
		"SV",
		"NI",
		"BZ",
		"PR"
	].includes(country)) return "Americas";
	return "Other";
}
function RouteSection({ routesWithFlights }) {
	const [showAll, setShowAll] = useState(false);
	const [search, setSearch] = useState("");
	const query = search.toLowerCase().trim();
	const topRoutes = routesWithFlights.slice(0, 10);
	const displayRoutes = showAll ? routesWithFlights : topRoutes;
	const filtered = query ? displayRoutes.filter((r) => {
		const name = routeDisplayName(r).toLowerCase();
		const iata = (routeIata(r) ?? "").toLowerCase();
		const icao = (r.destinationIcao ?? "").toLowerCase();
		return name.includes(query) || iata.includes(query) || icao.includes(query);
	}) : displayRoutes;
	const grouped = useMemo(() => {
		const map = /* @__PURE__ */ new Map();
		for (const r of filtered) {
			const region = routeRegion(r);
			const list = map.get(region) ?? [];
			list.push(r);
			map.set(region, list);
		}
		return Array.from(map.entries()).sort((a, b) => {
			if (a[0] === "Europe") return -1;
			if (b[0] === "Europe") return 1;
			return b[1].length - a[1].length;
		});
	}, [filtered]);
	return /* @__PURE__ */ jsxs("section", {
		className: "flex flex-col gap-4",
		children: [
			/* @__PURE__ */ jsx(ExhibitHeader, { children: "Exhibit D — Where You Can Escape To" }),
			/* @__PURE__ */ jsxs("span", {
				className: "font-grotesk text-[11px] font-bold text-zinc-100 tracking-wider uppercase",
				children: [routesWithFlights.length, " Routes Served"]
			}),
			/* @__PURE__ */ jsx("input", {
				type: "text",
				value: search,
				onChange: (e) => setSearch(e.target.value),
				placeholder: "Search destinations...",
				className: "w-full bg-zinc-900/50 border border-white/5 px-3 py-2 font-mono text-xs text-zinc-300 placeholder:text-zinc-600 focus:outline-none focus:border-yellow-400/30 transition-colors"
			}),
			/* @__PURE__ */ jsxs("div", {
				className: "max-h-[500px] overflow-y-auto scrollbar-thin",
				children: [grouped.map(([region, routes]) => /* @__PURE__ */ jsxs("div", {
					className: "mb-4",
					children: [/* @__PURE__ */ jsxs("div", {
						className: "flex items-center gap-2 mb-2 sticky top-0 bg-[#0a0a0b] py-1 z-10",
						children: [/* @__PURE__ */ jsx("span", {
							className: "font-grotesk text-[10px] font-bold text-zinc-500 tracking-[1.5px] uppercase",
							children: region
						}), /* @__PURE__ */ jsx("span", {
							className: "font-mono text-[10px] text-zinc-700",
							children: routes.length
						})]
					}), routes.map((r) => /* @__PURE__ */ jsxs("div", {
						className: "flex justify-between items-center py-2 border-b border-white/5 last:border-0",
						children: [
							/* @__PURE__ */ jsxs("span", {
								className: "font-mono text-xs text-zinc-400",
								children: [routeDisplayName(r), routeIata(r) ? ` (${routeIata(r)})` : ""]
							}),
							r.airlineName && /* @__PURE__ */ jsx("span", {
								className: "font-mono text-[10px] text-zinc-600 mx-4",
								children: r.airlineName
							}),
							/* @__PURE__ */ jsx("span", {
								className: "font-mono text-xs font-bold text-zinc-100 tabular-nums shrink-0",
								children: r.flightsPerMonth ?? "—"
							})
						]
					}, r.id))]
				}, region)), filtered.length === 0 && /* @__PURE__ */ jsxs("p", {
					className: "font-mono text-xs text-zinc-600 italic py-4",
					children: [
						"No routes matching \"",
						search,
						"\". Trapped."
					]
				})]
			}),
			routesWithFlights.length > 10 && !query && /* @__PURE__ */ jsx("button", {
				onClick: () => setShowAll(!showAll),
				className: "font-grotesk text-[11px] font-bold text-yellow-400 tracking-wider hover:text-yellow-300 transition-colors uppercase self-start",
				children: showAll ? `Show Top 10` : `Show All ${routesWithFlights.length} Routes`
			})
		]
	});
}
function Badge({ label, value, bright = false }) {
	return /* @__PURE__ */ jsxs("span", {
		className: "inline-flex items-center gap-1.5 bg-white/[0.03] px-2.5 py-1",
		children: [/* @__PURE__ */ jsx("span", {
			className: "font-grotesk text-[9px] font-bold text-zinc-600 tracking-wider uppercase",
			children: label
		}), /* @__PURE__ */ jsx("span", {
			className: `font-mono text-xs font-bold ${bright ? "text-zinc-100" : "text-zinc-400"}`,
			children: value
		})]
	});
}
//#endregion
export { AirportDetail as component };
