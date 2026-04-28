# Q2 Cross-Team Initiative

## Search Service (Search team)

We need to add full-text search across the catalogue. Customers
struggle to find products by descriptive attributes today. Implement a
search API backed by Postgres full-text search; the frontend already
has a search bar wired to a placeholder endpoint.

## Email Bounce Handling (Comms team)

Outgoing transactional emails are bouncing without any signal back to
the application. We need to capture bounce notifications from the
mail provider, mark the affected user accounts, and prevent further
sends until the address is updated.

## Data Export (Search team)

Customer support requested the ability to export search analytics as
CSV — top queries, no-result rates, click-through rates. The data
already exists in the analytics warehouse; this is a thin export
endpoint plus the corresponding admin-page button.
