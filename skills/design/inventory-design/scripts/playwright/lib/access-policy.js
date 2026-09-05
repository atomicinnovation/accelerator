// Reachability + scheme classification for navigation URLs, reimplemented from
// the Rust domain (design::host, design::host_reach, design::access_policy) so
// the daemon can classify each request without crossing back into Rust. The
// shared corpus at cli/design/tests/fixtures/host-classification-vectors.json
// holds this implementation and the Rust one to the same cases; either drifting
// fails CI.

const SCHEMES = [
  ['https://', 'https'],
  ['http://', 'http'],
];

function parseUrl(rawUrl) {
  for (const [prefix, scheme] of SCHEMES) {
    if (rawUrl.startsWith(prefix)) {
      const authority = rawUrl.slice(prefix.length).split(/[/?#]/)[0];
      return { ok: true, scheme, authority };
    }
  }
  return { ok: false };
}

function isControl(codePoint) {
  return codePoint <= 0x1f || (codePoint >= 0x7f && codePoint <= 0x9f);
}

// Decode %XX byte-wise, leaving a malformed escape as written, matching the
// Rust percent-decoder that lets a bad escape reach the later checks literally.
function percentDecode(raw) {
  const src = Buffer.from(raw, 'utf8');
  const out = [];
  let index = 0;
  while (index < src.length) {
    if (src[index] === 0x25 && index + 2 < src.length) {
      const hex = String.fromCharCode(src[index + 1], src[index + 2]);
      if (/^[0-9a-fA-F]{2}$/.test(hex)) {
        out.push(parseInt(hex, 16));
        index += 3;
        continue;
      }
    }
    out.push(src[index]);
    index += 1;
  }
  return Buffer.from(out).toString('utf8');
}

function stripPortAndBrackets(authority) {
  if (!authority.startsWith('[')) {
    return authority.split(':')[0];
  }
  const inside = authority.slice(1).split(']')[0];
  return inside.split('%')[0];
}

function isNumericLabel(label) {
  if (label.startsWith('0x')) {
    const hex = label.slice(2);
    return hex.length > 0 && /^[0-9a-f]+$/.test(hex);
  }
  return label.length > 0 && /^[0-9]+$/.test(label);
}

function looksNumeric(host) {
  return host.includes(':') || host.split('.').every(isNumericLabel);
}

function parseIpv4(text) {
  const octets = text.split('.');
  if (octets.length !== 4) return null;
  const bytes = new Uint8Array(4);
  for (let index = 0; index < 4; index += 1) {
    const octet = octets[index];
    if (!/^(0|[1-9][0-9]{0,2})$/.test(octet)) return null;
    const value = Number(octet);
    if (value > 255) return null;
    bytes[index] = value;
  }
  return bytes;
}

// A dotted-quad token is the address's trailing 32 bits, so it is only legal as
// the final token.
function tokensToUnits(tokens, allowTrailingV4) {
  const units = [];
  for (let index = 0; index < tokens.length; index += 1) {
    const token = tokens[index];
    if (token.includes('.')) {
      if (!(allowTrailingV4 && index === tokens.length - 1)) return null;
      const v4 = parseIpv4(token);
      if (!v4) return null;
      units.push((v4[0] << 8) | v4[1], (v4[2] << 8) | v4[3]);
    } else {
      if (!/^[0-9a-f]{1,4}$/.test(token)) return null;
      units.push(parseInt(token, 16));
    }
  }
  return units;
}

function parseIpv6(text) {
  const parts = text.split('::');
  if (parts.length > 2) return null;
  const compressed = parts.length === 2;
  const headTokens = parts[0] === '' ? [] : parts[0].split(':');
  const headUnits = tokensToUnits(headTokens, !compressed);
  if (!headUnits) return null;

  let units;
  if (compressed) {
    const tailTokens = parts[1] === '' ? [] : parts[1].split(':');
    const tailUnits = tokensToUnits(tailTokens, true);
    if (!tailUnits) return null;
    const zeros = 8 - headUnits.length - tailUnits.length;
    if (zeros < 1) return null;
    units = [...headUnits, ...new Array(zeros).fill(0), ...tailUnits];
  } else {
    if (headUnits.length !== 8) return null;
    units = headUnits;
  }

  const bytes = new Uint8Array(16);
  for (let index = 0; index < 8; index += 1) {
    bytes[index * 2] = (units[index] >> 8) & 0xff;
    bytes[index * 2 + 1] = units[index] & 0xff;
  }
  return bytes;
}

function parseAddress(canonical) {
  if (canonical.includes(':')) {
    const bytes = parseIpv6(canonical);
    return bytes ? { version: 6, bytes } : null;
  }
  const bytes = parseIpv4(canonical);
  return bytes ? { version: 4, bytes } : null;
}

export function canonicalise(authority) {
  if (authority.includes('@')) return { ok: false };
  const decoded = percentDecode(authority);
  for (const character of decoded) {
    if (isControl(character.codePointAt(0))) return { ok: false };
  }
  const lowered = decoded.toLowerCase();
  const stripped = stripPortAndBrackets(lowered);
  const canonical = stripped.endsWith('.') ? stripped.slice(0, -1) : stripped;
  if (canonical === '') return { ok: false };

  const address = parseAddress(canonical);
  if (!address && looksNumeric(canonical)) return { ok: false };
  return { ok: true, host: { canonical, address } };
}

function classifyV4(bytes) {
  const [first, second, third, fourth] = bytes;
  if (first === 0 && second === 0 && third === 0 && fourth === 0) {
    return 'unspecified';
  }
  if (first === 127) return 'loopback';
  if (
    first === 10 ||
    (first === 172 && second >= 16 && second <= 31) ||
    (first === 192 && second === 168)
  ) {
    return 'private';
  }
  if (first === 169 && second === 254) return 'link-local';
  if (
    (first >= 224 && first <= 239) ||
    first === 0 ||
    (first === 100 && second >= 64 && second < 128) ||
    (first === 192 && second === 0 && third === 0) ||
    (first === 198 && second >= 18 && second < 20) ||
    first >= 240
  ) {
    return 'reserved';
  }
  return 'public';
}

function toEmbeddedV4(bytes) {
  const firstTwelveZero = bytes.slice(0, 12).every(byte => byte === 0);
  const mapped =
    bytes.slice(0, 10).every(byte => byte === 0) &&
    bytes[10] === 0xff &&
    bytes[11] === 0xff;
  if (firstTwelveZero || mapped) {
    return Uint8Array.of(bytes[12], bytes[13], bytes[14], bytes[15]);
  }
  return null;
}

function embeddedV4(bytes) {
  const segment = index => (bytes[index * 2] << 8) | bytes[index * 2 + 1];
  if (segment(0) === 0x2002) {
    return Uint8Array.of(
      (segment(1) >> 8) & 0xff,
      segment(1) & 0xff,
      (segment(2) >> 8) & 0xff,
      segment(2) & 0xff
    );
  }
  if (segment(0) === 0x2001 && segment(1) === 0) {
    // RFC 4380 stores Teredo's mapped IPv4 address bitwise-inverted.
    const mapped = (((segment(6) << 16) >>> 0) | segment(7)) >>> 0;
    const inverted = ~mapped >>> 0;
    return Uint8Array.of(
      (inverted >>> 24) & 0xff,
      (inverted >>> 16) & 0xff,
      (inverted >>> 8) & 0xff,
      inverted & 0xff
    );
  }
  if (
    segment(0) === 0x0064 &&
    segment(1) === 0xff9b &&
    segment(2) === 0 &&
    segment(3) === 0 &&
    segment(4) === 0 &&
    segment(5) === 0
  ) {
    return Uint8Array.of(
      (segment(6) >> 8) & 0xff,
      segment(6) & 0xff,
      (segment(7) >> 8) & 0xff,
      segment(7) & 0xff
    );
  }
  return toEmbeddedV4(bytes);
}

function classifyV6(bytes) {
  if (bytes.every(byte => byte === 0)) return 'unspecified';
  // `::` and `::1` are IPv4-compatible forms as far as the embedded-address
  // unwrap is concerned, so they are answered before anything is unwrapped.
  if (bytes.slice(0, 15).every(byte => byte === 0) && bytes[15] === 1) {
    return 'loopback';
  }
  const embedded = embeddedV4(bytes);
  if (embedded) return classifyV4(embedded);

  const segment0 = (bytes[0] << 8) | bytes[1];
  if ((segment0 & 0xfe00) === 0xfc00) return 'private';
  if ((segment0 & 0xffc0) === 0xfe80) return 'link-local';
  if (bytes[0] === 0xff) return 'reserved';
  return 'public';
}

const LOOPBACK_NAME = 'localhost';

export function classifyHost(host) {
  if (!host.address) {
    return host.canonical === LOOPBACK_NAME ? 'loopback' : 'public';
  }
  return host.address.version === 4
    ? classifyV4(host.address.bytes)
    : classifyV6(host.address.bytes);
}

// The reach + scheme gate shared by classifyUrl and classifyLocation. Fails
// closed: any reach class not explicitly handled (a future reach) is refused as
// `malformed` rather than falling through to accept.
function judge(scheme, host, allowInternal, allowInsecureScheme) {
  const reach = classifyHost(host);
  if (reach === 'loopback') return { ok: true };
  if (reach === 'unspecified') {
    return { ok: false, classification: 'unspecified' };
  }
  if (reach === 'private' || reach === 'link-local' || reach === 'reserved') {
    return allowInternal ? { ok: true } : { ok: false, classification: reach };
  }
  if (reach !== 'public') return { ok: false, classification: 'malformed' };
  if (scheme === 'http' && !allowInsecureScheme) {
    return { ok: false, classification: 'insecure-scheme' };
  }
  return { ok: true };
}

// Judges a raw navigation URL under the invocation's allowances. Fails closed
// on every ambiguous input: missing allowances deny, and a canonicalisation
// failure (userinfo, control character, numeric encoding, a refused scheme) is
// `malformed`. The daemon never dials a URL this could not judge.
export function classifyUrl(rawUrl, allowances) {
  const { allowInternal = false, allowInsecureScheme = false } =
    allowances ?? {};
  const url = parseUrl(rawUrl);
  if (!url.ok) return { ok: false, classification: 'malformed' };
  const parsed = canonicalise(url.authority);
  if (!parsed.ok) return { ok: false, classification: 'malformed' };
  return judge(url.scheme, parsed.host, allowInternal, allowInsecureScheme);
}

// Judges a browser-resolved anchor destination — a concrete scheme and host,
// not a raw URL. The browser has already produced the host, so the raw-URL
// canonicalisation rejections that yield `malformed` on the navigate path
// cannot arise; both paths refuse the same internal reach classes. Returns only
// the boolean, since `links` folds a refusal into `same_origin: false`.
export function classifyLocation(location, allowances) {
  const { allowInternal = false, allowInsecureScheme = false } =
    allowances ?? {};
  const parsed = canonicalise(location.host);
  if (!parsed.ok) return { ok: false };
  const verdict = judge(
    location.scheme,
    parsed.host,
    allowInternal,
    allowInsecureScheme
  );
  return { ok: verdict.ok };
}

// The pure decision a route handler makes for one intercepted request. A
// non-navigation request (a subresource) and an allowed navigation both
// continue; a refused navigation aborts, carrying the classification and the
// refused URL. The main-frame gate lives in the handler, not here, so this stays
// a pure function of the request and the allowances.
export function classifyNavigationRequest(request, allowances) {
  if (!request.isNavigationRequest()) return { continue: true };
  const url = request.url();
  const verdict = classifyUrl(url, allowances);
  if (verdict.ok) return { continue: true };
  return { abort: true, classification: verdict.classification, url };
}
