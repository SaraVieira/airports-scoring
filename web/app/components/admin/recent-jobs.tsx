import { Table } from "lucide-react";
import { Card } from "../ui/card";
import { Link } from "@tanstack/react-router";
import { Button } from "../ui/button";
import {
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "../ui/table";
import { components } from "~/api/types";
import { JobStatusBadge } from "./job-status-badge";

export const RecentJobs = ({
  recentJobs,
}: {
  recentJobs: components["schemas"]["JobInfo"][];
}) => {
  return (
    <Card className="overflow-hidden flex flex-col">
      <div className="px-4 py-3 border-b border-border flex items-center justify-between">
        <span className="text-xs font-semibold tracking-wider text-muted-foreground uppercase">
          Recent Jobs
        </span>
        <Link to="/admin/jobs">
          <Button variant="ghost" size="xs">
            View all
          </Button>
        </Link>
      </div>
      <div className="flex-1 overflow-y-auto">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-20">Status</TableHead>
              <TableHead className="w-20">Airport</TableHead>
              <TableHead>Source</TableHead>
              <TableHead className="w-28">Progress</TableHead>
              <TableHead className="w-36">Started</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {recentJobs.length === 0 && (
              <TableRow>
                <TableCell
                  colSpan={5}
                  className="text-center text-muted-foreground py-8"
                >
                  No jobs yet
                </TableCell>
              </TableRow>
            )}
            {recentJobs.map((job) => (
              <TableRow key={job.id}>
                <TableCell>
                  <JobStatusBadge status={job.status} />
                </TableCell>
                <TableCell className="font-mono text-xs">
                  {job.progress.currentAirport || job.airports[0] || "—"}
                </TableCell>
                <TableCell className="text-xs text-muted-foreground">
                  {job.progress.currentSource ||
                    job.sources.slice(0, 3).join(", ") ||
                    "all"}
                </TableCell>
                <TableCell>
                  <div className="flex items-center gap-2">
                    <div className="w-16 h-1 rounded-full bg-muted overflow-hidden">
                      <div
                        className="h-full rounded-full bg-yellow-500 transition-all"
                        style={{
                          width: `${
                            job.progress.airportsTotal > 0
                              ? (job.progress.airportsCompleted /
                                  job.progress.airportsTotal) *
                                100
                              : 0
                          }%`,
                        }}
                      />
                    </div>
                    <span className="text-xs text-muted-foreground tabular-nums">
                      {job.progress.airportsCompleted}/
                      {job.progress.airportsTotal}
                    </span>
                  </div>
                </TableCell>
                <TableCell className="text-xs text-muted-foreground">
                  {job.startedAt
                    ? new Date(job.startedAt).toLocaleString()
                    : "—"}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>
    </Card>
  );
};
