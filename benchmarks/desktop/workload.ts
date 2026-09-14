/** One deterministic offline dataset, embedded identically in all six apps. */
export const workload = {
  id: "issue-tracker-v1",
  width: 1100,
  height: 720,
  records: 1000,
  pageSize: 100,
  readyTitle: "Issue tracker — ready",
} as const;

export function createIssues() {
  const titles = [
    "Keep offline changes when reconnecting",
    "Improve keyboard navigation in search",
    "Show upload progress for large attachments",
    "Restore the last selected workspace",
    "Add a compact layout for the activity feed",
    "Make notification preferences easier to find",
    "Support bulk updates in the issue list",
    "Preserve draft notes when switching projects",
    "Speed up the first workspace sync",
    "Export filtered issues as a spreadsheet",
    "Improve contrast in status labels",
    "Handle interrupted downloads gracefully",
  ];
  const projects = ["Desktop", "Mobile", "Platform", "Design"];
  const owners = ["Alex Chen", "Sam Rivera", "Morgan Lee", "Jamie Park", "Taylor Brooks"];
  return Array.from({ length: workload.records }, (_, index) => ({
    id: `APP-${1001 + index}`,
    title: titles[index % titles.length]!,
    project: projects[index % projects.length]!,
    owner: owners[index % owners.length]!,
    priority: ["High", "Medium", "Low"][index % 3]!,
    status: index % 5 === 4 ? "Done" : index % 3 === 1 ? "In progress" : "Open",
    description: `Customers use ${projects[index % projects.length]!.toLowerCase()} throughout the day and expect their workspace to stay predictable. ${titles[index % titles.length]} without interrupting their current work.\n\nAcceptance criteria: preserve existing data, handle keyboard and pointer input, and provide a clear result when the operation completes. Include the offline and empty states in the review.\n\nReported by ${owners[index % owners.length]} during the September product review. Related customer conversation: CX-${2001 + index}.`,
    notes:
      "Reproduce with a fresh workspace, then verify the existing workflow still works. Share the result with the team before release.",
  }));
}
