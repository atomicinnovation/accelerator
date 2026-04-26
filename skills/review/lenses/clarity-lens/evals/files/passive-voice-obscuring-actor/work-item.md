---
title: "Export report as PDF"
type: story
status: ready
---

# Export Report as PDF

## Summary

Reports can currently only be viewed in the browser. This work item adds the
ability for a user to export a report as a downloadable PDF file.

## Context

Users have requested the ability to download reports as PDFs for sharing with
stakeholders who do not have system access. The reports module already produces
HTML output; the PDF export should use this as its source.

## Requirements

1. A download button must be added to the report view page by the frontend.
2. When the button is clicked, the report is rendered to PDF.
3. The PDF is generated on the server side.
4. The generated file is streamed to the browser as a download.
5. If an error occurs during generation, the user is shown an error message.
6. The PDF must be formatted to A4 paper size.
7. Headers and footers are included, showing the report title and page numbers.

## Acceptance Criteria

- When the user clicks the download button, a PDF file is downloaded to their
  device.
- The file is formatted to A4 paper size with headers and footers.
- If PDF generation fails, an error message is displayed in the report view.

## Technical Notes

The team has agreed to use a headless browser for server-side rendering.
Puppeteer is the preferred library. Generation should be triggered by an HTTP
request to a new `/reports/:id/pdf` endpoint.
