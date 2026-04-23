---
title: "Multi-tenancy support"
type: epic
status: draft
priority: high
---

# Multi-Tenancy Support

## Summary

The platform currently serves a single organisation per deployment. This epic
adds multi-tenancy support so that a single deployment can serve multiple
organisations with strict data isolation between tenants.

## Context

Three enterprise prospects have indicated that multi-tenancy is a prerequisite
for signing contracts. The current single-tenant architecture requires a
separate deployment per customer, which the infrastructure team has flagged as
unsustainable beyond 20 customers. The sales team's Q3 target includes at least
two enterprise accounts, each requiring multi-tenancy.

## Acceptance Criteria

- Each tenant's data is isolated and inaccessible to other tenants at the
  database layer.
- A platform administrator can create, suspend, and delete tenants without
  engineering involvement.
- Tenant-specific branding (logo, colour scheme) can be configured per tenant
  by their own administrators.
- All existing single-tenant functionality continues to work correctly for each
  tenant in isolation.

## Technical Notes

Row-level security in PostgreSQL is the preferred isolation mechanism. Each
tenant will be assigned a UUID that is added to all data rows. The API gateway
will extract the tenant ID from the authentication token and inject it into each
request context.
