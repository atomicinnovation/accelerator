// The request-body field names the client writes and the daemon reads for
// header-mode auth. One shared constant so a rename changes both sides at once
// and cannot silently un-authenticate the crawl across the JS boundary.

export const LOCATION_URL_FIELD = 'location_url';
export const AUTH_HEADER_FIELD = 'auth_header';
