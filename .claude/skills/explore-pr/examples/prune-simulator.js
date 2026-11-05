(() => {
'use strict';
const MIN = 60, H = 3600, DAY = 86400;
const AGE_BACKSTOP = H, CLAIM_WINDOW = 14 * DAY, REFRESH = CLAIM_WINDOW / 10;
const PLATFORMS = ['darwin-arm64', 'darwin-x64', 'linux-arm64', 'linux-x64'];
const PLAT = `(${PLATFORMS.join('|')})`;
const HEX = '[0-9a-f]{64}';
const GEN_SUFFIX = '0011223344556677';
const RES = 'cli/launcher/src/launch/outbound/resolve/';
const R = (p, line) => ({ path: /^(cli|bin|docs-site)\//.test(p) ? p : RES + p, line });
const REFS = {
  gateCold: R('retention/root_retention.rs', 118), gateVerb: R('retention/root_retention.rs', 170), gateDefault: R('retention/root_retention.rs', 44),
  startUp: R('cli/launcher/src/launch/start_up.rs', 72), claimYoung: R('retention/version_claims.rs', 45), rootLock: R('retention/root_prune.rs', 123),
  classify: R('retention/root_entries.rs', 79), claimsList: R('retention/version_claims.rs', 106), claimsLstat: R('retention/version_claims.rs', 138),
  claimFresh: R('retention/version_claims.rs', 159), suffix: R('retention/single_file.rs', 117), pairRemove: R('retention/single_file.rs', 373),
  shim: R('retention/single_file.rs', 57), staleClaims: R('retention/single_file.rs', 401), residue: R('retention/residue.rs', 59), aged: R('retention_windows.rs', 23),
  survey: R('tree/prune.rs', 229), visit: R('tree/prune.rs', 121), reap: R('tree/reap.rs', 137), unclaimed: R('tree/prune.rs', 402), removeTree: R('tree/prune.rs', 516),
  sidecars: R('tree/prune.rs', 458), orphanLock: R('tree/prune.rs', 487), staleTree: R('tree/prune.rs', 575), treeClaim: R('tree/claims.rs', 76), treeLstat: R('tree/claims.rs', 183),
  pointer: R('tree/layout.rs', 315), lease: R('tree/lease.rs', 42), verb: R('cli/launcher/src/launch/cache.rs', 155), safe: R('ownership/safe_root.rs', 108),
};

/* ---------- Names ---------- */
const TOK = {};
function tok(t) {
  if (!TOK[t]) { let h = 2166136261 >>> 0, out = ''; for (let i = 0; out.length < 64; i++) { h ^= (t.charCodeAt(i % t.length) + i * 31) & 255; h = Math.imul(h, 16777619) >>> 0; out += (h >>> 0).toString(16).padStart(8, '0'); } TOK[t] = out.slice(0, 64); }
  return TOK[t];
}
const expand = s => String(s).replace(/\{(\w+)\}/g, (_, t) => tok(t));
const genName = t => `${t.artifact}-${t.platform}-{${t.digest}}-${t.layout || 1}-${GEN_SUFFIX}`;
const clampAge = a => (a == null ? null : Math.max(0, a));
const aged = (a, type) => (type === undefined || type === 'file' || type === 'dir') && a != null && clampAge(a) >= AGE_BACKSTOP;

function classifyRoot(name, listed) {
  const n = expand(name);
  let m;
  if ((m = n.match(new RegExp(`^accelerator-verify-${PLAT}-(${HEX})\\.staging\\.([1-9]\\d*)$`)))) return { kind: 'ShimStaging', platform: m[1], digest: m[2], pid: +m[3] };
  if ((m = n.match(new RegExp(`^accelerator-verify-${PLAT}-(${HEX})$`)))) return { kind: 'ShimCopy', platform: m[1], digest: m[2] };
  if ((m = n.match(new RegExp(`^accelerator-launcher-(.+)-${PLAT}$`))) && listed.has(n + '.minisig')) return { kind: 'LauncherEntry', version: m[1], platform: m[2] };
  if ((m = n.match(new RegExp(`^([^.].*)-(${HEX})$`))) && listed.has(n + '.minisig')) return { kind: 'SubBinary', prefix: m[1], digest: m[2] };
  if ((m = n.match(/^\.tmp-launcher-([1-9]\d*)(\.minisig)?$/))) return { kind: 'LauncherStaging', pid: +m[1] };
  if ((m = n.match(new RegExp(`^\\.tmp-([^.].*-${HEX})-(\\d+)-(\\d+)(\\.minisig)?$`)))) return { kind: 'StoreStaging', stem: m[1] };
  if (/^\.accelerator-probe-\d+(-\d+)?$/.test(n)) return { kind: 'Probe' };
  if ((m = n.match(new RegExp(`^\\.accelerator-lock-${PLAT}$`)))) return { kind: 'BootstrapLock', platform: m[1] };
  return { kind: 'Unrecognised' };
}
const KIND_LABEL = { ShimStaging: 'shim staging', ShimCopy: 'shim copy', LauncherEntry: 'launcher entry', SubBinary: 'sub-binary', LauncherStaging: 'launcher staging', StoreStaging: 'store staging', Probe: 'probe', BootstrapLock: 'bootstrap lock', Unrecognised: 'kept name', Signature: 'signature' };
const belongsTo = (prefix, v) => prefix.endsWith('-' + v) && prefix.length > v.length + 1;
function parseClaim(name) {
  const i = name.lastIndexOf('.');
  if (i <= 0) return null;
  const v = name.slice(0, i), id = name.slice(i + 1);
  return /^[0-9a-f]{16}$/.test(id) ? { version: v, id } : null;
}
function pointerTarget(t) {
  const want = `${t.artifact}-${t.platform}-${tok(t.digest)}-`;
  const contents = {
    current: `${t.artifact}-${t.platform}-${tok(t.digest)}-1-${GEN_SUFFIX}`,
    'other-layout': `${t.artifact}-${t.platform}-${tok(t.digest)}-2-${GEN_SUFFIX}`,
    'other-artifact': `${t.artifact === 'driver' ? 'browser' : 'driver'}-${t.platform}-${tok(t.digest)}-1-${GEN_SUFFIX}`,
    '../victim': '../victim', '/abs': '/Users/me/victim',
  }[t.pointer];
  if (t.pointer === 'symlink') return { kind: 'Malformed', why: 'the pointer is a symlink; the bounded read refuses it', readable: false };
  if (t.pointer === 'oversize') return { kind: 'Malformed', why: 'the pointer is over 256 bytes; the bounded read refuses it', readable: false };
  const c = contents.trim();
  if (/[\\/]/.test(c)) return { kind: 'Malformed', why: `contents \`${c}\` hold a path separator; never joined`, readable: true, contents: c };
  if (c.startsWith(want + '1-') && /-[0-9a-f]{16}$/.test(c)) return { kind: 'Current', readable: true, contents: c };
  if (c.startsWith(want) && /-\d+-[0-9a-f]{16}$/.test(c)) return { kind: 'OtherLayout', why: 'names this digest under another layout number; kept', readable: true, contents: c };
  return { kind: 'Malformed', why: 'names another artifact\'s generation; never followed', readable: true, contents: c };
}

/* ---------- Engine ---------- */
function run(input) {
  const w = JSON.parse(JSON.stringify(input));
  const out = { dec: {}, events: [], report: { locks: [], claims: [], untrusted: [], removals: [] }, reclaimed: 0, created: [], removed: new Set(), stdout: null, exit: null, refused: null, notes: {} };
  const dec = (key, fate, rule, ref, trace) => { out.dec[key] = { fate, rule, ref, trace: trace || (out.dec[key] && out.dec[key].trace) || [] }; if (fate === 'removed') out.removed.add(key); };
  const tr = (key, step) => { (out.dec[key] = out.dec[key] || { fate: 'kept', rule: '', trace: [] }).trace.push(step); };
  const ev = (text, ref) => out.events.push({ text, ref });
  const alive = key => !out.removed.has(key);
  const verb = w.trigger === 'verb';
  const patience = verb ? 'Wait' : 'Try';
  const window = verb && w.olderThan != null && w.olderThan !== '' ? Number(w.olderThan) : CLAIM_WINDOW;
  const all = keysOf(w);
  all.forEach(k => dec(k, 'kept', 'not reached by this run', null, []));

  if (w.root === 'unidentified') {
    all.forEach(k => dec(k, 'kept', 'the cache root could not be identified; nothing runs', REFS.gateVerb));
    if (verb) { out.refused = 'error: the cache root is unavailable'; out.exit = 1; }
    ev('Start-up could not identify the cache root; UnidentifiedRetention claims and prunes nothing', R('retention/root_retention.rs', 199));
    return finish(w, out);
  }
  if (w.root === 'relocated') startUpClaim(w, out, ev, dec, tr);
  else ev('Default root: no version claim is written', REFS.gateDefault);
  keysOf(w).forEach(k => { if (!out.dec[k]) dec(k, 'kept', 'written at start-up by this launcher', REFS.startUp, (out.dec[k] || {}).trace); });
  if (w.trigger === 'start-up') { all.forEach(k => { if (!out.dec[k].rule || out.dec[k].rule === 'not reached by this run') dec(k, 'kept', 'start-up only claims; nothing is pruned', REFS.startUp); }); return finish(w, out); }

  if (w.root === 'default') {
    keysOf(w).filter(k => k.startsWith('e:') || k.startsWith('c:')).forEach(k => dec(k, 'kept', 'the default root has no root prune; ADR-0063 governs it', REFS.gateDefault));
    if (w.trigger === 'cold-store') { ev('Cold store in the default root: no prune runs', REFS.gateDefault); keysOf(w).filter(k => !k.startsWith('e:') && !k.startsWith('c:')).forEach(k => dec(k, 'kept', 'a cold store in the default root prunes nothing', REFS.gateDefault)); return finish(w, out); }
    const T = treeCtx(w, out, dec, tr, ev, alive, { regime: 'PinnedOnHost', patience, window });
    if (w.trigger === 'cold-materialise') { T.materialiseReapOnly(w.materialise || 'browser'); }
    else T.prune();
    return finish(w, out);
  }

  if (!w.safe) {
    const reason = unsafeText(w);
    keysOf(w).forEach(k => { if (out.dec[k].rule === 'not reached by this run') dec(k, 'kept', 'SafePruneRoot refused this root; nothing is removed', REFS.safe); });
    if (verb) { out.refused = `not pruning: ${reason}`; out.exit = 1; }
    else ev(`warn (once per process): not pruning the cache root: ${reason}`, R('retention/root_retention.rs', 122));
    return finish(w, out);
  }

  const T = treeCtx(w, out, dec, tr, ev, alive, { regime: 'WholeTree', patience, window });
  if (w.trigger === 'cold-materialise') T.materialiseHeld(w.materialise || 'browser');
  rootPrune(w, out, { patience, window, dec, tr, ev, alive });
  T.prune();
  return finish(w, out);
}
function unsafeText(w) {
  const p = '/Users/me/.cache/accelerator';
  return ({
    'shared-group': `${p} is group-writable by the shared group staff; run chmod g-w ${p}`,
    'world': `${p} is world-writable; run chmod 700 ${p}, or set ACCELERATOR_CACHE_DIR to a per-user directory on a native filesystem`,
    'ancestor': `/Users/me/.cache is owned by uid 502, who could redirect a prune; if that account is trusted, set ACCELERATOR_CACHE_TRUSTED_OWNERS=502`,
    'acl': `${p} has an access ACL that may let another user write to it; set ACCELERATOR_CACHE_DIR to a per-user directory on a native filesystem`,
  })[w.unsafeReason || 'shared-group'];
}
function finish(w, out) {
  out.w = w;
  if (w.trigger === 'verb' && !out.refused) out.exit = (out.report.locks.length || out.report.removals.length) ? 1 : 0;
  if (w.trigger === 'verb' && !out.refused) out.stdout = `reclaimed ${out.reclaimed} entries`;
  return out;
}
function keysOf(w) {
  const k = [];
  w.entries.forEach(e => k.push('e:' + e.id));
  w.claims.forEach(c => k.push('c:' + c.id));
  w.trees.forEach(t => { if (t.pointer !== 'none') k.push(`t:${t.id}:ptr`); if (t.generation) k.push(`t:${t.id}:gen`); if (t.sidecars) k.push(`t:${t.id}:side`); });
  w.treeClaims.forEach(c => k.push('tc:' + c.id));
  w.locks.forEach(l => k.push('l:' + l.id));
  w.temps.forEach(x => k.push('x:' + x.id));
  w.foreign.forEach(f => k.push('f:' + f.id));
  return k;
}

function startUpClaim(w, out, ev, dec, tr) {
  const name = `${w.running.version}.${w.running.id}`;
  const c = w.claims.find(x => x.name === name);
  if (c && c.type === 'file' && c.owner === 'self' && c.age != null && clampAge(c.age) < REFRESH) {
    ev(`Start-up: claims/${name} is younger than 1.4 d, so it is not rewritten`, REFS.claimYoung);
    tr('c:' + c.id, 'start-up: young enough; not rewritten');
    return;
  }
  if (w.claimsDir === 'absent') { w.claimsDir = 'readable'; ev('Start-up: created claims/ with mode 0700', REFS.startUp); }
  if (w.claimsDir === 'untrusted') { ev('Start-up: claims/ is not a trusted directory; the claim is not written (debug log only)', REFS.startUp); return; }
  if (w.claimsDir === 'unreadable' || w.claimsDir === 'unsearchable') { ev(`Start-up: writing claims/${name} fails without search permission on claims/; logged at debug only, the command carries on`, REFS.startUp); return; }
  if (c && c.type === 'symlink') { ev(`Start-up: claims/${name} is a symlink; O_NOFOLLOW refuses it and the claim is not written`, REFS.startUp); return; }
  if (c) { c.age = 0; c.type = 'file'; c.owner = 'self'; tr('c:' + c.id, 'start-up: rewritten with mtime = now'); }
  else { const id = nextId(w); w.claims.push({ id, name, age: 0, type: 'file', owner: 'self', fresh: true }); out.created.push('c:' + id); }
  ev(`Start-up: wrote claims/${name} before any lookup`, REFS.startUp);
}
function nextId(w) { w._seq = (w._seq || 1000) + 1; return w._seq; }

function rootPrune(w, out, { patience, window, dec, tr, ev, alive }) {
  const rootKeys = [...w.entries.map(e => 'e:' + e.id), ...w.claims.map(c => 'c:' + c.id)];
  const skipAll = rule => rootKeys.forEach(k => dec(k, 'kept', rule, REFS.rootLock));
  if (w.rootLock === 'held') {
    if (patience === 'Try') { ev('Root lock held by another prune: single-file prune and the residue sweep are skipped, silently', REFS.rootLock); skipAll('the root lock was held; a cold path skips the root prune rather than wait'); return; }
    ev('Root lock held by another prune: the verb waits, then takes it', REFS.rootLock);
  } else if (w.rootLock === 'error' || w.rootLock === 'foreign') {
    const detail = w.rootLock === 'error' ? 'No locks available (os error 77)' : 'the lock file is a symlink or not owned by you';
    out.report.locks.push(`could not lock <root>/.accelerator-prune.lock: ${detail}`);
    ev(`Root lock failed (${detail}): single-file prune and residue sweep skipped`, REFS.rootLock);
    skipAll('the root lock could not be taken; nothing at the root level is removed');
    return;
  } else ev(`Took .accelerator-prune.lock (${patience === 'Try' ? 'LockPatience::Try' : 'LockPatience::Wait'})`, REFS.rootLock);

  const listed = new Set(w.entries.map(e => expand(e.name)));
  ev(`Listed the root once: ${w.entries.length} names`, REFS.classify);
  const cls = new Map(w.entries.map(e => [e.id, classifyRoot(e.name, listed)]));
  const runningShim = w.running.shimReadable ? tok(w.running.shim) : null;

  let claimsState = 'Read';
  const claims = [];
  if (w.claimsDir === 'unreadable' || w.claimsDir === 'untrusted') {
    claimsState = 'Unreadable';
    out.report.claims.push('claims unreadable (permission denied) in <root>/claims; kept everything they could protect');
    ev(`claims/ is ${w.claimsDir === 'untrusted' ? 'not a trusted directory' : 'unreadable'}: listing is Unreadable(permission denied); every versioned entry and claim removal is skipped`, REFS.claimsList);
  } else if (w.claimsDir !== 'absent') {
    w.claims.forEach(c => {
      const p = parseClaim(c.name);
      const key = 'c:' + c.id;
      if (!p) { tr(key, 'name does not parse as <version>.<16 hex>: ignored'); dec(key, 'kept', 'not a claim name: ignored, never removed', REFS.claimsList); return; }
      if (w.claimsDir === 'unsearchable') { tr(key, 'lstat fails (no search permission on claims/)'); dec(key, 'kept', 'dropped silently: its lstat failed, so it protects nothing (R1-06)', REFS.claimsLstat); return; }
      const valid = c.type === 'file' && c.owner === 'self';
      const fresh = valid && (c.age == null || clampAge(c.age) < window);
      claims.push({ c, key, ...p, valid, fresh });
      tr(key, `parsed: version ${p.version}, launcher ${p.id}`);
      tr(key, valid ? `valid; age ${fmtAge(c.age)} ${fresh ? '<' : '≥'} window ${fmtAge(window)}: ${fresh ? 'fresh' : 'stale'}` : 'not a regular file owned by you: protects nothing');
    });
    ev(`Read claims/: ${claims.length} listed${w.claimsDir === 'unsearchable' ? ', every one dropped because its lstat failed (R1-06); the listing still reports Read' : ''}`, w.claimsDir === 'unsearchable' ? REFS.claimsLstat : REFS.claimsList);
  } else ev('claims/ absent: no version is fresh except the running one', REFS.claimsList);
  const freshSet = new Set(claims.filter(x => x.fresh).map(x => x.version));
  freshSet.add(w.running.version);
  const otherLive = [...freshSet].filter(v => v !== w.running.version);

  const sigOf = e => w.entries.find(s => expand(s.name) === expand(e.name) + '.minisig');
  const versioned = [];
  const sorted = [...w.entries].sort((a, b) => expand(a.name) < expand(b.name) ? -1 : 1);
  sorted.forEach(e => { if (/\.minisig$/.test(e.name)) dec('e:' + e.id, 'kept', 'a signature alone is a kept name; it goes only with its entry', REFS.classify); });
  sorted.forEach(e => {
    const k = 'e:' + e.id, c = cls.get(e.id);
    if (/\.minisig$/.test(e.name) && c.kind === 'Unrecognised') return;
    tr(k, `classified as ${KIND_LABEL[c.kind]}`);
    const age = e.age;
    const removeFile = (rule, ref) => { dec(k, 'removed', rule, ref); out.reclaimed++; };
    switch (c.kind) {
      case 'SubBinary': case 'LauncherEntry': {
        const sig = sigOf(e);
        const versions = c.kind === 'SubBinary' ? [...freshSet].filter(v => belongsTo(c.prefix, v)) : (freshSet.has(c.version) ? [c.version] : []);
        const rec = { e, removed: false, belongs: v => c.kind === 'SubBinary' ? belongsTo(c.prefix, v) : c.version === v };
        versioned.push(rec);
        if (claimsState !== 'Read') { dec(k, 'kept', 'claims unreadable: versioned entries are skipped', REFS.claimsList); return; }
        if (versions.length) { const v = versions[0]; dec(k, 'kept', v === w.running.version ? 'the running version is always protected' : `protected: version ${v} has a fresh claim`, REFS.suffix); tr(k, c.kind === 'SubBinary' ? `name ends in -${v} before the digest` : `launcher version ${v} is fresh`); if (sig) dec('e:' + sig.id, 'kept', 'kept with its entry', REFS.suffix); return; }
        tr(k, 'no fresh version matches by suffix');
        if (!(aged(age, e.type) && sig && aged(sig.age, sig.type))) { dec(k, 'kept', 'unclaimed, but it or its signature is younger than AGE_BACKSTOP (1 h)', REFS.aged); return; }
        tr(k, 'entry and signature both aged ≥ 1 h');
        if (sig) { dec('e:' + sig.id, 'removed', 'removed first, before its entry (counts 0)', REFS.pairRemove); }
        removeFile(c.kind === 'SubBinary' ? 'unclaimed aged sub-binary: signature, then entry' : 'unclaimed aged launcher entry: signature, then entry', REFS.pairRemove);
        rec.removed = true;
        if (otherLive.length === 0 && !w.claims.some(x => { const p = parseClaim(x.name); return p && rec.belongs(p.version); })) out.notes[k] = 'If a launcher from before version claims still uses this root, its next dispatch re-fetches; offline it fails (accepted risk AR3, R1-12).';
        return;
      }
      case 'ShimCopy': {
        if (!runningShim) { dec(k, 'kept', 'the running shim for this platform cannot be hashed: keep every copy', REFS.shim); return; }
        if (c.digest === runningShim) { dec(k, 'kept', "the running plugin's own shim copy", REFS.shim); return; }
        if (!aged(age, e.type)) { dec(k, 'kept', 'another release\'s shim copy, younger than 1 h', REFS.shim); return; }
        removeFile('aged shim copy of another release; claims do not protect shim copies', REFS.shim);
        if (otherLive.length) out.notes[k] = `Version ${otherLive.join(', ')} is live in this root. If its shim bytes differ, a bootstrap reusing this copy fails with fail_integrity (R1-04, accepted risk AR4).`;
        return;
      }
      case 'ShimStaging': case 'LauncherStaging': {
        if ((w.livePids || []).includes(c.pid)) { dec(k, 'kept', `pid ${c.pid} is alive: a bootstrap is still staging`, REFS.residue); return; }
        if (!aged(age, e.type)) { dec(k, 'kept', 'staging younger than 1 h', REFS.aged); return; }
        removeFile('aged staging of a finished bootstrap (pid dead)', REFS.residue); return;
      }
      case 'StoreStaging': {
        if (!aged(age, e.type)) { dec(k, 'kept', 'store staging younger than 1 h', REFS.aged); return; }
        removeFile('aged store staging; protection is not consulted', REFS.pairRemove); return;
      }
      case 'Probe': case 'BootstrapLock': return;
      default: dec(k, 'kept', 'a name the plugin does not produce: always kept', REFS.classify);
    }
  });

  if (claimsState === 'Read') {
    claims.forEach(x => {
      if (!x.valid) { dec(x.key, 'kept', 'not a regular file owned by you: protects nothing, never removed', REFS.claimFresh); return; }
      if (x.fresh) { dec(x.key, 'kept', `fresh claim: age < ${fmtAge(window)}`, REFS.claimFresh); return; }
      const twin = claims.some(y => y !== x && y.version === x.version && y.fresh);
      const mine = versioned.filter(r => r.belongs(x.version));
      const allGone = mine.every(r => r.removed);
      if (twin || allGone) { dec(x.key, 'removed', twin ? 'stale; another claim keeps the version fresh' : (mine.length ? 'stale, and every entry of its version is gone: the last claim goes last' : 'stale, and its version has no entries left'), REFS.staleClaims); tr(x.key, 're-checked immediately before unlink: still valid and stale'); out.reclaimed++; }
      else dec(x.key, 'kept', 'stale, but its version still has entries: a last claim goes only after them', REFS.staleClaims);
    });
  } else w.claims.forEach(c => dec('c:' + c.id, 'kept', 'claims unreadable: no claim is removed', REFS.claimsList));

  sorted.forEach(e => {
    const k = 'e:' + e.id, c = cls.get(e.id);
    if (c.kind === 'Probe') {
      if (e.type === 'file' && aged(e.age, e.type)) { dec(k, 'removed', 'aged probe', REFS.residue); out.reclaimed++; }
      else dec(k, 'kept', e.type !== 'file' ? 'a probe that is not a regular file is kept' : 'probe younger than 1 h', REFS.residue);
    }
    if (c.kind === 'BootstrapLock') {
      if (e.type !== 'dir' || e.owner !== 'self') { dec(k, 'kept', 'not a trusted directory: kept', REFS.residue); return; }
      if (!aged(e.age, 'dir')) { dec(k, 'kept', 'lock directory younger than 1 h', REFS.residue); return; }
      const pid = e.pid || 'missing';
      if (pid === 'live') { dec(k, 'kept', 'its pid is alive: a bootstrap holds it', REFS.residue); return; }
      if (pid === 'garbage') { dec(k, 'kept', 'pid is not a positive integer (or over 64 bytes): keep', REFS.residue); return; }
      tr(k, `pid ${pid}; judged twice with the same result`);
      if (e.extra) { dec(k, 'failed', 'pid removed, but rmdir fails: the directory holds more than pid', REFS.residue); out.report.removals.push('could not remove <root>/' + expand(e.name) + ': directory not empty'); return; }
      dec(k, 'removed', `stale bootstrap lock (pid ${pid}): pid, then rmdir`, REFS.residue); out.reclaimed++;
    }
  });
  ev('Released the root lock before any tree lock is taken', REFS.rootLock);
}

function treeCtx(w, out, dec, tr, ev, alive, { regime, patience, window }) {
  const relocated = regime === 'WholeTree';
  const keyOf = (a, p) => `${a}/${p}`;
  const pin = a => w.pins[a];
  const lockFor = (a, p) => w.locks.find(l => l.artifact === a && l.platform === p && alive('l:' + l.id));
  let claimsState = null, claims = [];
  let reapRan = false;
  const surveyed = { ok: false };
  function survey() {
    if (surveyed.done) return surveyed.ok;
    surveyed.done = true;
    const treeKeys = keysOf(w).filter(k => /^(t|tc|l|x|f):/.test(k));
    if (w.treesDir === 'absent') { ev('trees/ absent: tree prune ends', REFS.survey); return false; }
    if (w.treesDir === 'untrusted') { out.report.untrusted.push('not pruning through <root>/trees: another user could write to it'); treeKeys.forEach(k => dec(k, 'kept', 'trees/ is not a trusted directory: nothing under it is touched', REFS.survey)); ev('trees/ untrusted: tree prune ends', REFS.survey); return false; }
    if (w.treesDir === 'unlistable') { out.report.removals.push('could not remove <root>/trees: permission denied'); treeKeys.forEach(k => dec(k, 'kept', 'trees/ could not be listed', REFS.survey)); ev('trees/ could not be listed: a removal failure', REFS.survey); return false; }
    w.foreign.forEach(f => dec('f:' + f.id, 'kept', 'not the tree name grammar: always kept', REFS.pointer));
    if (w.treeClaimsDir === 'unreadable') { claimsState = 'Unreadable'; out.report.claims.push('claims unreadable (permission denied) in <root>/trees/claims; kept everything they could protect'); }
    else if (w.treeClaimsDir === 'untrusted') { claimsState = 'Untrusted'; out.report.untrusted.push('not pruning through <root>/trees/claims: another user could write to it'); }
    else {
      claimsState = 'Read';
      if (w.treeClaimsDir !== 'absent') w.treeClaims.forEach(c => {
        const k = 'tc:' + c.id;
        if (w.treeClaimsDir === 'no-search') { tr(k, 'lstat fails (no search permission on trees/claims/)'); dec(k, 'kept', 'not listed: its lstat failed, so it protects nothing and is never swept (R1-08)', REFS.treeLstat); return; }
        const valid = c.type === 'file' && c.owner === 'self';
        claims.push({ c, k, valid, protects: valid && (c.age == null || clampAge(c.age) < window), stale: valid && c.age != null && clampAge(c.age) >= window });
      });
    }
    if (claimsState !== 'Read') w.treeClaims.forEach(c => dec('tc:' + c.id, 'kept', `trees/claims/ ${claimsState === 'Untrusted' ? 'untrusted' : 'unreadable'}: claims are neither consulted nor swept`, REFS.survey));
    ev(`Surveyed trees/ (${regime}): claims ${claimsState}${w.treeClaimsDir === 'no-search' ? ' and empty, because every claim\'s lstat failed (R1-08)' : ''}`, w.treeClaimsDir === 'no-search' ? REFS.treeLstat : REFS.survey);
    w.trees.forEach(t => {
      if (t.pointer !== 'none') dec(`t:${t.id}:ptr`, 'kept', 'pointer kept', REFS.unclaimed);
      if (t.generation) dec(`t:${t.id}:gen`, 'kept', 'generation kept', REFS.unclaimed);
      if (t.sidecars) dec(`t:${t.id}:side`, 'kept', 'sidecars kept with their generation', REFS.sidecars);
    });
    w.locks.forEach(l => dec('l:' + l.id, 'kept', 'tree lock kept: removed only once its artifact has no work and it is 1 h old', REFS.orphanLock));
    w.temps.forEach(x => dec('x:' + x.id, 'kept', 'temp residue kept: only a reap removes it', REFS.reap));
    if (claimsState !== 'Read') w.trees.forEach(t => {
      const why = `trees/claims/ ${claimsState === 'Untrusted' ? 'untrusted' : 'unreadable'}: no unclaimed tree is removed`;
      if (t.pointer !== 'none') dec(`t:${t.id}:ptr`, 'kept', why, REFS.survey);
      if (t.generation) dec(`t:${t.id}:gen`, 'kept', why, REFS.survey);
    });
    surveyed.ok = true;
    return true;
  }
  const liveness = t => (t.heldByUs ? 'Held' : t.lease || 'Free');
  const hasWork = (a, p) => w.trees.some(t => t.artifact === a && t.platform === p && ((t.pointer !== 'none' && alive(`t:${t.id}:ptr`)) || (t.generation && alive(`t:${t.id}:gen`)))) || w.temps.some(x => x.artifact === a && x.platform === p && alive('x:' + x.id));
  const sidecarOnly = (a, p) => w.trees.some(t => t.artifact === a && t.platform === p && !t.generation && t.sidecars && alive(`t:${t.id}:side`));
  function referenced() {
    const s = new Set();
    w.trees.forEach(t => { if (t.pointer !== 'none' && alive(`t:${t.id}:ptr`)) { const pt = pointerTarget(t); if (pt.readable) s.add(pt.contents); } });
    return s;
  }
  function reap(under) {
    reapRan = true;
    ev(`Orphan reap ran once, under ${under}'s lock, across every artifact`, REFS.reap);
    w.temps.forEach(x => {
      const k = 'x:' + x.id;
      if (!alive(k)) return;
      const owner = keyOf(x.artifact, x.platform), lock = lockFor(x.artifact, x.platform) || out.created.find(c => c.artifact === x.artifact && c.platform === x.platform);
      let rel;
      if (owner === under) rel = 'under';
      else if (!lock) rel = 'none';
      else if (lock.errors) rel = 'error';
      else if (lock.held) rel = 'held';
      else rel = 'free';
      tr(k, `owner ${owner}: ${{ under: 'the lock this reap runs under', none: 'no lock file', error: 'lock probe errors', held: 'lock held by another process', free: 'lock probed and taken' }[rel]}`);
      if (rel === 'held') { dec(k, 'kept', "its owner's lock is held by another process", REFS.reap); return; }
      if (rel === 'error') { dec(k, 'kept', "its owner's lock probe failed (warning)", REFS.reap); return; }
      if (x.kind === 'archive') {
        if (rel === 'none') { dec(k, 'kept', 'a partial archive is kept while its owner has no lock file', REFS.reap); return; }
        if (x.digest === pin(x.artifact)) { dec(k, 'kept', "the partial archive of the owner's pinned digest", REFS.reap); return; }
      }
      dec(k, 'removed', 'reaped temp residue (no age gate)', REFS.reap); out.reclaimed++;
    });
    const refs = referenced();
    w.trees.forEach(t => {
      const k = `t:${t.id}:gen`;
      if (!t.generation || !alive(k)) return;
      const name = expand(genName(t));
      if ((t.layout || 1) !== 1) { tr(k, `reap: layout ${t.layout} is never reaped`); return; }
      if (refs.has(name)) { tr(k, 'reap: a pointer names it'); return; }
      if (liveness(t) === 'Held') { tr(k, 'reap: unreferenced but its lease is held'); dec(k, 'kept', 'unreferenced, but its lease is held', REFS.reap); return; }
      if (!aged(t.genAge, 'dir')) { tr(k, 'reap: unreferenced but younger than 1 h'); dec(k, 'kept', 'unreferenced, but younger than 1 h', REFS.reap); return; }
      dec(k, 'removed', 'reaped: an unreferenced generation, unheld and aged', REFS.reap); out.reclaimed++;
      if (t.sidecars && alive(`t:${t.id}:side`)) dec(`t:${t.id}:side`, 'removed', 'removed with the reaped generation (not counted; errors ignored)', REFS.reap);
    });
  }
  function unclaimed(a, p) {
    w.trees.filter(t => t.artifact === a && t.platform === p && t.pointer !== 'none').sort((x, y) => x.digest < y.digest ? -1 : 1).forEach(t => {
      const pk = `t:${t.id}:ptr`, gk = `t:${t.id}:gen`, sk = `t:${t.id}:side`;
      if (!alive(pk)) return;
      const pt = pointerTarget(t);
      tr(pk, `pointer classified ${pt.kind}${pt.why ? ': ' + pt.why : ''}`);
      if (pt.kind !== 'Current') { dec(pk, 'kept', pt.kind === 'OtherLayout' ? 'names another layout: kept' : `malformed pointer kept; ${pt.why}`, REFS.pointer); return; }
      const keepAll = (rule, ref) => { dec(pk, 'kept', rule, ref); if (t.generation && alive(gk)) dec(gk, 'kept', rule, ref); if (t.sidecars && alive(sk)) dec(sk, 'kept', 'kept with their generation', ref); };
      if (t.digest === pin(a)) { keepAll("the running launcher's pinned digest", REFS.unclaimed); return; }
      const prot = claims.find(c => c.c.digest === t.digest && c.protects && alive(c.k));
      if (prot) { keepAll(`claimed: ${prot.c.id} is ${fmtAge(prot.c.age)} old, inside ${fmtAge(window)}`, REFS.treeClaim); return; }
      const live = liveness(t);
      if (live === 'Held') { keepAll(t.heldByUs ? 'this dispatch holds its lease' : 'another process holds its lease', REFS.lease); return; }
      if (live === 'Unknown' && !aged(t.genAge, 'dir')) { keepAll('lease state unknown and the generation is younger than 1 h', REFS.lease); return; }
      tr(pk, `unpinned, unclaimed, lease ${live}${live === 'Free' ? ' (any age)' : ' and aged'}; claims re-read: still unprotected`);
      dec(pk, 'removed', 'pointer unlinked first (not counted)', REFS.removeTree);
      if (t.race) {
        if (t.generation && alive(gk)) dec(gk, 'kept', 'a dispatch leased it after the check: the exclusive lease is refused and the generation stays; the pointer is already gone', REFS.removeTree);
        if (t.sidecars && alive(sk)) dec(sk, 'kept', 'kept: the exclusive lease was refused', REFS.removeTree);
        claims.filter(c => c.c.digest === t.digest && alive(c.k)).forEach(c => dec(c.k, 'kept', 'kept: the exclusive lease was refused', REFS.removeTree));
        out.notes[gk] = 'Next prune, once the dispatch exits and the generation is 1 h old: unreferenced, so the reap removes it. That installation re-fetches.';
        ev(`${a}/${p} ${'{' + t.digest + '}'}: pointer removed, but a dispatch took a shared lease, so the exclusive lease failed and the tree stays`, REFS.removeTree);
        return;
      }
      if (t.generation && alive(gk)) { dec(gk, 'removed', 'unclaimed tree: removed under an exclusive lease on its .lease', REFS.removeTree); out.reclaimed++; }
      if (t.sidecars && alive(sk)) dec(sk, 'removed', 'removed with the tree (not counted)', REFS.removeTree);
      claims.filter(c => c.c.digest === t.digest && alive(c.k)).forEach(c => dec(c.k, 'removed', 'removed with the tree, from the re-read listing (not counted)', REFS.removeTree));
    });
  }
  function orphanSidecars(a, p) {
    w.trees.filter(t => t.artifact === a && t.platform === p && t.sidecars && !(t.generation && alive(`t:${t.id}:gen`)) && alive(`t:${t.id}:side`)).forEach(t => {
      const sk = `t:${t.id}:side`;
      if (t.generation) return;
      if (liveness(t) === 'Held') { dec(sk, 'kept', 'orphan sidecars whose lease is held', REFS.sidecars); return; }
      if (!aged(t.sideAge, 'file')) { dec(sk, 'kept', 'orphan sidecars younger than 1 h', REFS.sidecars); return; }
      dec(sk, 'removed', 'orphan sidecars: no generation, unheld, aged (+1 each)', REFS.sidecars); out.reclaimed += 3;
    });
  }
  function orphanLock(a, p) {
    const l = lockFor(a, p);
    if (!l) return;
    if (hasWork(a, p)) { dec('l:' + l.id, 'kept', 'its artifact still has work', REFS.orphanLock); return; }
    if (!aged(l.age, 'file')) { dec('l:' + l.id, 'kept', 'no work left, but the lock is younger than 1 h', REFS.orphanLock); return; }
    dec('l:' + l.id, 'removed', 'orphan tree lock: no work left, aged, confirmed by a fresh listing', REFS.orphanLock); out.reclaimed++;
  }
  function artifacts() {
    if (!relocated) return Object.keys(w.pins).map(a => [a, w.running.platform]);
    const s = new Set();
    w.trees.forEach(t => s.add(keyOf(t.artifact, t.platform)));
    w.temps.forEach(x => s.add(keyOf(x.artifact, x.platform)));
    w.locks.forEach(l => s.add(keyOf(l.artifact, l.platform)));
    return [...s].sort().map(k => k.split('/'));
  }
  function arm(a, p) {
    const l = lockFor(a, p), name = `.tmp-${a}-${p}.lock`;
    if (l && l.errors) { out.report.locks.push(`could not lock <root>/trees/${name}: No locks available (os error 77)`); dec('l:' + l.id, 'failed', 'the lock call errored (ENOLCK)', REFS.visit); ev(`${a}/${p}: lock call failed`, REFS.visit); return 'Failed'; }
    if (hasWork(a, p)) {
      if (l && l.held) { if (patience === 'Try') { ev(`${a}/${p}: tree lock held elsewhere; skipped with no report (Try)`, REFS.visit); return 'Skipped'; } ev(`${a}/${p}: tree lock held elsewhere; the verb waits for it`, REFS.visit); }
      if (!l) { const id = nextId(w); const nl = { id, artifact: a, platform: p, age: 0, created: true }; w.locks.push(nl); out.created.push('l:' + id); dec('l:' + id, 'kept', 'created by this prune to take the tree lock; young', REFS.visit); ev(`${a}/${p}: created and took ${name}`, REFS.visit); }
      else ev(`${a}/${p}: took ${name}`, REFS.visit);
      return 'Taken';
    }
    if (relocated && l) {
      if (l.held && patience === 'Try') { ev(`${a}/${p}: no work, lock held elsewhere; skipped`, REFS.visit); return 'Skipped'; }
      ev(`${a}/${p}: no work; took the existing lock without creating one`, REFS.visit);
      return 'Taken';
    }
    if (relocated && sidecarOnly(a, p)) { ev(`${a}/${p}: sidecars only; no lock call`, REFS.visit); return 'LockFree'; }
    return 'Skipped';
  }
  return {
    prune() {
      if (!survey()) return;
      ev(`Tree prune: ${regime}, ${patience === 'Try' ? 'LockPatience::Try' : 'LockPatience::Wait'}`, REFS.visit);
      for (const [a, p] of artifacts()) {
        const r = arm(a, p);
        if (r === 'Taken') {
          if (!reapRan) reap(`${a}/${p}`);
          if (claimsState === 'Read') unclaimed(a, p);
          if (relocated) { orphanSidecars(a, p); orphanLock(a, p); }
        } else if (r === 'LockFree' && relocated) orphanSidecars(a, p);
      }
      const orphanGen = w.trees.some(t => t.generation && alive(`t:${t.id}:gen`) && !referenced().has(expand(genName(t))));
      if (!reapRan && (w.temps.some(x => alive('x:' + x.id)) || orphanGen)) { const first = Object.keys(w.pins)[0]; ev(`Fallback reap under the first pinned artifact's lock (${first})`, R('tree/prune.rs', 166)); reap(`${first}/${w.running.platform}`); }
      if (relocated && claimsState === 'Read') {
        claims.forEach(c => { if (!alive(c.k)) return; if (c.stale) { dec(c.k, 'removed', `stale tree claim: ≥ ${fmtAge(window)} old, re-checked before unlink`, REFS.staleTree); out.reclaimed++; } else if (c.protects) dec(c.k, 'kept', `fresh tree claim (< ${fmtAge(window)})`, REFS.treeClaim); else dec(c.k, 'kept', 'not a regular file owned by you: never swept', REFS.treeClaim); });
        ev('Stale tree-claim sweep, holding no lock', REFS.staleTree);
      } else if (claimsState === 'Read') claims.forEach(c => { if (alive(c.k)) dec(c.k, 'kept', 'the default root never sweeps tree claims', REFS.staleTree); });
    },
    materialiseHeld(x) {
      if (!survey()) return;
      ev(`Materialised ${x} under its tree lock`, R('tree/after_materialise.rs', 1));
      reap(`${x}/${w.running.platform}`);
      if (claimsState === 'Read') unclaimed(x, w.running.platform);
      orphanSidecars(x, w.running.platform);
      ev(`Released ${x}'s tree lock; the full root prune follows`, R('retention/root_prune.rs', 187));
      reapRan = false;
    },
    materialiseReapOnly(x) {
      if (!survey()) return;
      ev(`Default root: materialised ${x}; the reap runs under its lock, and nothing else`, REFS.gateDefault);
      reap(`${x}/${w.running.platform}`);
    },
  };
}

function commit(input, out) {
  const w = out.w;
  const gone = k => out.removed.has(k);
  w.entries = w.entries.filter(e => !gone('e:' + e.id));
  w.claims = w.claims.filter(c => !gone('c:' + c.id));
  w.trees.forEach(t => {
    if (gone(`t:${t.id}:ptr`)) t.pointer = 'none';
    if (gone(`t:${t.id}:gen`)) t.generation = false;
    if (gone(`t:${t.id}:side`)) t.sidecars = false;
    t.race = false;
  });
  w.trees = w.trees.filter(t => t.pointer !== 'none' || t.generation || t.sidecars);
  w.treeClaims = w.treeClaims.filter(c => !gone('tc:' + c.id));
  w.locks = w.locks.filter(l => !gone('l:' + l.id));
  w.temps = w.temps.filter(x => !gone('x:' + x.id));
  return w;
}

function fmtAge(a) {
  if (a == null) return 'unknown';
  if (a < 0) return 'future';
  if (a < H) return `${Math.round(a / MIN)} min`;
  if (a < DAY * 2 && a % DAY !== 0) return `${+(a / H).toFixed(1)} h`;
  return `${+(a / DAY).toFixed(1)} d`;
}
function parseAge(s) {
  s = String(s).trim().toLowerCase();
  if (s === '?' || s === 'unknown') return null;
  if (s === 'future') return -H;
  const m = s.match(/^(-?\d+(?:\.\d+)?)\s*(s|sec|m|min|h|d)?$/);
  if (!m) return undefined;
  const n = parseFloat(m[1]), u = m[2] || 'h';
  return Math.round(n * ({ s: 1, sec: 1, m: MIN, min: MIN, h: H, d: DAY })[u]);
}

/* ---------- Presets ---------- */
function base(extra) {
  let id = 0;
  const st = { root: 'relocated', safe: true, unsafeReason: 'shared-group', trigger: 'cold-store', materialise: 'browser', olderThan: '', running: { version: '1.26.0', id: 'c0ffee00c0ffee00', platform: 'darwin-arm64', shim: 'hC', shimReadable: true }, pins: { browser: 'C', driver: 'E' }, rootLock: 'free', claimsDir: 'readable', treesDir: 'absent', treeClaimsDir: 'ok', livePids: [], entries: [], claims: [], trees: [], treeClaims: [], locks: [], temps: [], foreign: [], _seq: 0 };
  Object.assign(st, extra || {});
  const nid = () => ++id;
  const api = {
    st,
    file(name, age, o) { st.entries.push({ id: nid(), name, age, type: 'file', owner: 'self', ...(o || {}) }); return api; },
    pair(name, age) { return api.file(name, age).file(name + '.minisig', age); },
    dir(name, age, o) { st.entries.push({ id: nid(), name, age, type: 'dir', owner: 'self', ...(o || {}) }); return api; },
    claim(name, age, o) { st.claims.push({ id: nid(), name, age, type: 'file', owner: 'self', ...(o || {}) }); return api; },
    tree(artifact, digest, o) { st.treesDir = 'ok'; st.trees.push({ id: nid(), artifact, platform: 'darwin-arm64', digest, pointer: 'current', generation: true, genAge: 3 * H, sidecars: true, sideAge: 3 * H, lease: 'Free', heldByUs: false, race: false, layout: 1, ...(o || {}) }); return api; },
    tclaim(digest, idHex, age, o) { st.treeClaims.push({ id: nid(), digest, lid: idHex, age, type: 'file', owner: 'self', ...(o || {}) }); return api; },
    lock(artifact, age, o) { st.locks.push({ id: nid(), artifact, platform: 'darwin-arm64', age, held: false, errors: false, ...(o || {}) }); return api; },
    temp(kind, artifact, digest, age, o) { st.temps.push({ id: nid(), kind, artifact, platform: 'darwin-arm64', digest, age, ...(o || {}) }); return api; },
    foreign(name, age) { st.foreign.push({ id: nid(), name, age }); return api; },
    done() { st._seq = id + 100; return st; },
  };
  return api;
}
const d = n => n * DAY, h = n => n * H, m = n => n * MIN;
const PRESETS = [
  { id: 'abc', title: 'A/B/C upgrade', tags: ['root', 'trees'], about: 'The work item\'s headline scenario. Versions A (1.24.0) and B (1.25.0) used this root 20 days ago; C (1.26.0) has just fetched `visualiser` on a cold store. The root also holds two unclaimed `browser` trees and C\'s own.', expect: 'Every A and B name goes, with their claims, staging, shim copies, probe and dead bootstrap lock: 13 at the root. Trees A and B go, C\'s stays: 2 more. 15 in all; the dispatch proceeds to `exec`.', expectCount: 15,
    build: () => base().pair('vcs-1.24.0-{dA}', d(20)).pair('accelerator-launcher-1.24.0-darwin-arm64', d(20)).file('.tmp-vcs-1.24.0-{dA}-4242-0', d(20)).file('accelerator-verify-darwin-arm64-{hA}', d(20))
      .pair('vcs-1.25.0-{dB}', d(20)).pair('accelerator-launcher-1.25.0-darwin-arm64', d(20)).file('.tmp-vcs-1.25.0-{dB}-4343-0', d(20)).file('accelerator-verify-darwin-arm64-{hB}', d(20))
      .pair('visualiser-1.26.0-{dC}', 0).pair('accelerator-launcher-1.26.0-darwin-arm64', 0).file('.tmp-visualiser-1.26.0-{dC}-5151-0', h(2)).file('accelerator-verify-darwin-arm64-{hC}', 0)
      .file('.accelerator-probe-4242', h(2)).dir('.accelerator-lock-darwin-arm64', h(2), { pid: 'dead' }).file('.accelerator-unverified.log', d(3)).file('.accelerator-prune.lock', d(1))
      .claim('1.24.0.0123456789abcdef', d(20)).claim('1.25.0.fedcba9876543210', d(20))
      .tree('browser', 'A').tclaim('A', '9c4b1e0f2a7d5e83', d(20)).tree('browser', 'B').tree('browser', 'C', { genAge: h(1), sideAge: h(1) }).tclaim('C', '1a2b3c4d5e6f7081', h(1)).lock('browser', d(30)).done() },
  { id: 'live2', title: 'Two live versions', tags: ['root', 'R1-04'], about: 'Version 1.25.0 started two days ago and still runs beside 1.26.0, with a different vendored shim. A cold store by 1.26.0 prunes.', expect: '1.25.0\'s entries stay, protected by its fresh claim. Its staged shim copy goes: claims protect entries, not shim copies. If 1.25.0 bootstraps from that copy now it fails with `fail_integrity` (accepted risk).', expectCount: 1,
    build: () => base().claim('1.25.0.fedcba9876543210', d(2)).pair('vcs-1.25.0-{dB}', d(3)).pair('accelerator-launcher-1.25.0-darwin-arm64', d(3)).file('accelerator-verify-darwin-arm64-{hB}', d(3)).file('accelerator-verify-darwin-arm64-{hC}', d(1)).pair('visualiser-1.26.0-{dC}', 0).done() },
  { id: 'preclaim', title: 'Pre-claim launcher', tags: ['root', 'R1-12'], about: 'A 1.24.0 pre-release, built before version claims, shares the root and never writes a claim. 1.26.0 prunes on a cold store.', expect: 'Its entries older than an hour go; the 30-minute-old `browser` entry stays. That install re-fetches on its next dispatch, and its re-fetched entries go again once an hour old.', expectCount: 2,
    build: () => base().pair('vcs-1.24.0-pre.88-{dA}', h(2)).pair('accelerator-launcher-1.24.0-pre.88-darwin-arm64', h(2)).pair('browser-1.24.0-pre.88-{dB}', m(30)).done() },
  { id: 'unreadable', title: 'Claims unreadable', tags: ['root'], about: '`claims/` is mode 0000, so it cannot be listed. `accelerator cache prune` runs.', expect: 'Every versioned entry is kept and the verb says why; unversioned residue still goes. Exit 0, `reclaimed 3 entries`.', expectCount: 3,
    build: () => base({ trigger: 'verb', claimsDir: 'unreadable' }).pair('vcs-1.0.0-{dA}', d(20)).file('.tmp-vcs-1.0.0-{dA}-11-0', h(2)).file('accelerator-verify-darwin-arm64-{hA}', h(2)).file('.accelerator-probe-11', h(2)).done() },
  { id: 'unsearchable', title: 'Claims unsearchable (R1-06)', tags: ['root', 'R1-06'], about: '`claims/` is mode 0600: listable, but every `lstat` inside it fails. 1.25.0 used the root yesterday. The verb runs.', expect: 'As implemented, the listing still reports Read with no claims, so 1.25.0\'s entry goes although it is claimed, with no warning. With R1-06 fixed this would behave like "Claims unreadable".', expectCount: 1,
    build: () => base({ trigger: 'verb', claimsDir: 'unsearchable' }).claim('1.25.0.fedcba9876543210', d(1)).pair('vcs-1.25.0-{dB}', d(3)).pair('vcs-1.26.0-{dC}', d(3)).done() },
  { id: 'lockheld', title: 'Root lock held', tags: ['root', 'trees'], about: 'Another process holds `.accelerator-prune.lock`. Switch the trigger between cold store and the verb to compare `Try` with `Wait`.', expect: 'Cold store: the unclaimed entry and the probe stay, the tree prune still runs and removes the unclaimed `driver` tree (1). Verb: it waits, then removes all three (3).', expectCount: 1,
    build: () => base({ rootLock: 'held' }).pair('vcs-1.0.0-{dA}', d(20)).file('.accelerator-probe-11', h(2)).tree('driver', 'B', { genAge: d(20), sideAge: d(20) }).done() },
  { id: 'olderthan', title: '--older-than 1 d (R1-27)', tags: ['root', 'R1-27'], about: '`cache prune --older-than 86400`. 1.25.0 last started 20 hours ago, but its claim was written 30 hours ago and not rewritten, because a claim younger than 1.4 d is left alone.', expect: 'The 30-hour claim is older than the 1-day window, so 1.25.0 counts as unused: its entry and claim go. Without the flag both stay. Documented in `internals.md`.', expectCount: 2,
    build: () => base({ trigger: 'verb', olderThan: 86400 }).claim('1.25.0.fedcba9876543210', h(30)).pair('vcs-1.25.0-{dB}', d(3)).pair('vcs-1.26.0-{dC}', d(3)).done() },
  { id: 'pointers', title: 'Hostile pointers', tags: ['trees'], about: 'Pointers holding `../victim`, an absolute path, another artifact\'s generation, a symlink, and another layout number, beside one honest, unclaimed pointer. The verb runs.', expect: 'Only the honest tree goes (1). No victim directory is touched; the `driver` and layout-2 generations stay because a pointer names them or the layout is never reaped.', expectCount: 1,
    build: () => base({ trigger: 'verb' }).dir('victim', d(5)).tree('browser', 'D1', { pointer: '../victim', generation: false, sidecars: false }).tree('browser', 'D2', { pointer: '/abs', generation: false, sidecars: false }).tree('browser', 'D3', { pointer: 'other-artifact', generation: false, sidecars: false })
      .tree('driver', 'D3', { pointer: 'none', sidecars: false }).tree('browser', 'D4', { pointer: 'symlink', generation: false, sidecars: false }).tree('browser', 'D5', { pointer: 'other-layout', layout: 2, sidecars: false }).tree('browser', 'D6').done() },
  { id: 'leaserace', title: 'Lease during removal', tags: ['trees'], about: 'An unclaimed, aged `browser` tree whose lease is free when probed. A dispatch takes a shared lease after the prune\'s check and before the pointer unlink.', expect: 'The pointer goes, the exclusive lease is refused, and the generation stays: `reclaimed 0 entries`. Keep the result, advance an hour, and run again: the reap removes the now-unreferenced generation.', expectCount: 0,
    build: () => base({ trigger: 'verb' }).tree('browser', 'A', { race: true }).done() },
  { id: 'tclaims-unreadable', title: 'Tree claims unreadable', tags: ['trees'], about: '`trees/claims/` is mode 0000. A sibling install claimed `browser@S` an hour ago. A `legacy` temp generation is left from a crash.', expect: 'Every claimable tree is kept and the verb says so; the temp residue is still reaped (1). Exit 0.', expectCount: 1,
    build: () => base({ trigger: 'verb', treeClaimsDir: 'unreadable' }).tree('browser', 'S').tclaim('S', '9c4b1e0f2a7d5e83', h(1)).tree('browser', 'C').temp('temp-gen', 'legacy', 'F', h(3)).done() },
  { id: 'tclaims-nosearch', title: 'Tree claims unsearchable (R1-08)', tags: ['trees', 'R1-08'], about: 'Same root, but `trees/claims/` is mode 0600: listable, every `lstat` fails.', expect: 'As implemented, `browser@S` is removed although a sibling claimed it an hour ago, and nothing is reported. Its claim is never listed, so it is never unlinked either.', expectCount: 2,
    build: () => base({ trigger: 'verb', treeClaimsDir: 'no-search' }).tree('browser', 'S').tclaim('S', '9c4b1e0f2a7d5e83', h(1)).tree('browser', 'C').temp('temp-gen', 'legacy', 'F', h(3)).done() },
  { id: 'scope', title: 'Default against relocated', tags: ['trees'], about: 'The same trees in both kinds of root: an old host `browser`, a retired `legacy` tree with its lock, a `linux-x64` tree, orphan sidecars, a stale claim on no tree, and a foreign directory. Switch the root kind.', expect: 'Default root (`PinnedOnHost`): only the old host `browser` tree goes (1). Relocated (`WholeTree`): 8 go, and a young lock is created for `linux-x64`.', expectCount: 8,
    build: () => base({ trigger: 'verb' }).tree('browser', 'old').tree('legacy', 'L').lock('legacy', h(2)).tree('browser', 'X', { platform: 'linux-x64' }).tree('browser', 'G', { pointer: 'none', generation: false }).tclaim('Z', '0123456789abcdef', d(20)).foreign('backup-2021-1-final', h(3)).done() },
  { id: 'treelock', title: 'Held tree lock', tags: ['trees'], about: 'Another process holds `browser`\'s tree lock. `browser@A` and `driver@B` are aged and unclaimed. Compare the cold store with the verb, and try making the lock call error.', expect: 'Cold store: `browser` is skipped with no report; `driver@B` goes (1). Verb: waits, then both trees go, and so does `browser`\'s lock, now orphaned and a day old (3). With the lock call erroring: `driver@B` still goes and the verb exits non-zero.', expectCount: 1,
    build: () => base().tree('browser', 'A').tree('driver', 'B').lock('browser', d(1), { held: true }).done() },
  { id: 'unsafe', title: 'Unsafe root', tags: ['root'], about: 'The relocated root is group-writable by a shared group, so `SafePruneRoot` refuses it. The verb runs.', expect: 'Nothing is removed. The verb prints `not pruning: …` naming the directory and the fix, and exits non-zero. The version claim is still written.', expectCount: 0,
    build: () => base({ trigger: 'verb', safe: false }).pair('vcs-1.24.0-{dA}', d(20)).file('.accelerator-probe-11', h(2)).done() },
];

window.__simEngine = { run, commit, PRESETS, classifyRoot, parseAge, fmtAge, expand, tok, pointerTarget, keysOf, genName };

/* ---------- UI ---------- */
const ExplorePR = window.ExplorePR;
if (!ExplorePR) return;
const { esc, md } = ExplorePR.api;
let presetId = 'abc';
let state = PRESETS[0].build();
let last = null;
let openRow = null;
const pretty = s => esc(s).replace(/\{(\w+)\}/g, '<span class="tok">‹$1›</span>');

function rows(st) {
  const r = [];
  const tname = t => `${t.artifact}-${t.platform}-{${t.digest}}`;
  st.entries.forEach(e => r.push({ key: 'e:' + e.id, group: '<root>/', name: e.name + (e.type === 'dir' ? '/' : ''), age: e.age, kind: kindOfEntry(st, e), item: e, list: 'entries' }));
  st.claims.forEach(c => r.push({ key: 'c:' + c.id, group: '<root>/claims/', name: c.name, age: c.age, kind: 'version claim', item: c, list: 'claims' }));
  st.trees.forEach(t => {
    if (t.pointer !== 'none') r.push({ key: `t:${t.id}:ptr`, group: '<root>/trees/', name: `${tname(t)}.ref`, age: t.genAge, kind: `pointer (${t.pointer})`, item: t, list: 'trees' });
    if (t.generation) r.push({ key: `t:${t.id}:gen`, group: '<root>/trees/', name: `${genName(t)}/`, age: t.genAge, kind: `generation · lease ${t.heldByUs ? 'held by us' : t.lease}`, item: t, list: 'trees' });
    if (t.sidecars) r.push({ key: `t:${t.id}:side`, group: '<root>/trees/', name: `${genName(t)}.{sealed,sealed.sig,lease}`, age: t.sideAge, kind: t.generation ? 'sidecars' : 'orphan sidecars', item: t, list: 'trees' });
  });
  st.locks.forEach(l => r.push({ key: 'l:' + l.id, group: '<root>/trees/', name: `.tmp-${l.artifact}-${l.platform}.lock`, age: l.age, kind: `tree lock${l.held ? ' · held' : ''}${l.errors ? ' · errors' : ''}`, item: l, list: 'locks' }));
  st.temps.forEach(x => r.push({ key: 'x:' + x.id, group: '<root>/trees/', name: x.kind === 'archive' ? `.tmp-${x.artifact}-${x.platform}-{${x.digest}}.archive` : `.tmp-${x.artifact}-${x.platform}-{${x.digest}}-1-89abcdef01234567${x.kind === 'temp-ptr' ? '.ref' : '/'}`, age: x.age, kind: x.kind === 'archive' ? 'partial archive' : x.kind === 'temp-ptr' ? 'temp pointer' : 'temp generation', item: x, list: 'temps' }));
  st.foreign.forEach(f => r.push({ key: 'f:' + f.id, group: '<root>/trees/', name: f.name + '/', age: f.age, kind: 'foreign', item: f, list: 'foreign' }));
  st.treeClaims.forEach(c => r.push({ key: 'tc:' + c.id, group: '<root>/trees/claims/', name: `{${c.digest}}.${c.lid}`, age: c.age, kind: 'tree claim', item: c, list: 'treeClaims' }));
  return r;
}
function kindOfEntry(st, e) {
  const listed = new Set(st.entries.map(x => expand(x.name)));
  const c = classifyRoot(e.name, listed);
  if (c.kind === 'Unrecognised' && /\.minisig$/.test(e.name) && listed.has(expand(e.name).replace(/\.minisig$/, ''))) return 'signature';
  let k = KIND_LABEL[c.kind];
  if (c.kind === 'BootstrapLock') k += ` · pid ${e.pid || 'missing'}${e.extra ? ' + more' : ''}`;
  if ((c.kind === 'ShimStaging' || c.kind === 'LauncherStaging') && (st.livePids || []).includes(c.pid)) k += ' · pid alive';
  return k;
}

function vSim() {
  const p = PRESETS.find(x => x.id === presetId);
  return `<div class="view-head"><div class="eyebrow">Prune simulator</div><h1>What a prune removes, and why</h1>
    <p class="lede">A model of root prune and tree prune, transcribed rule by rule from the code at head with a source line for every decision. Pick a scenario, change any input, and every name's fate updates. Where the model and the code disagree, the code wins, and the disagreement deserves a note.</p></div>
    <div class="presets" role="group" aria-label="Scenarios">${PRESETS.map(x => `<button type="button" class="btn" data-preset="${x.id}" aria-pressed="${x.id === presetId}">${esc(x.title)}</button>`).join('')}</div>
    ${p ? `<div class="grid-2"><div class="panel stack"><div class="eyebrow">Scenario</div><div class="md">${md(p.about)}</div></div><div class="panel stack"><div class="eyebrow">What the code should do</div><div class="md">${md(p.expect)}</div><div class="small" id="sim-match"></div></div></div>` : ''}
    <div class="sim-layout">
      <div class="sim-controls panel" id="sim-controls"></div>
      <div class="stack" style="gap:16px;min-width:0">
        <div class="panel stack" style="gap:6px"><div class="row"><span class="eyebrow">Claim ages against the window</span><span class="spacer"></span><span class="small muted">● version claim · ■ tree claim</span></div><div class="ruler" id="sim-ruler"></div></div>
        <div class="row"><button class="btn primary" type="button" id="sim-commit">Keep the result</button><button class="btn" type="button" id="sim-reset">Reset scenario</button><span class="small muted" id="sim-summary" aria-live="polite"></span></div>
        <div class="panel stack"><div class="eyebrow">Output</div><div class="report" id="sim-report"></div></div>
        <div class="table-wrap"><table class="listing" id="sim-listing"></table></div>
        <details class="panel"><summary>Sequence of events</summary><ol class="small" id="sim-events" style="margin:0;padding-left:1.3em;display:grid;gap:4px"></ol></details>
      </div>
    </div>`;
}
function sel(id, value, opts) { return `<select id="${id}">${opts.map(([v, l]) => `<option value="${esc(v)}" ${String(value) === String(v) ? 'selected' : ''}>${esc(l)}</option>`).join('')}</select>`; }
function controls() {
  const s = state;
  const versions = [...new Set([s.running.version, ...s.claims.map(c => (parseClaim(c.name) || {}).version).filter(Boolean)])];
  return `
    <div class="field"><label for="sc-root">Cache root</label>${sel('sc-root', s.root, [['relocated', 'Relocated (ACCELERATOR_CACHE_DIR)'], ['default', 'Default (plugin bin/)'], ['unidentified', 'Could not be identified']])}</div>
    ${s.root === 'relocated' ? `<div class="field"><label><input type="checkbox" id="sc-safe" ${s.safe ? 'checked' : ''}> Passes SafePruneRoot</label>${s.safe ? '' : sel('sc-unsafe', s.unsafeReason, [['shared-group', 'group-writable by a shared group'], ['world', 'world-writable'], ['ancestor', 'ancestor owned by another uid'], ['acl', 'ACL may grant write']])}</div>` : ''}
    <div class="field"><label for="sc-trigger">Trigger</label>${sel('sc-trigger', s.trigger, [['cold-store', 'Cold path: after a store (Try)'], ['cold-materialise', 'Cold path: after a tree materialise (Try)'], ['verb', 'accelerator cache prune (Wait)'], ['start-up', 'Start-up only (claim, no prune)']])}</div>
    ${s.trigger === 'verb' ? `<div class="field"><label for="sc-older">--older-than</label>${sel('sc-older', s.olderThan, [['', 'not given (14 d)'], [String(H), '3600 (1 h)'], [String(DAY), '86400 (1 d)'], [String(7 * DAY), '604800 (7 d)'], ['0', '0']])}</div>` : ''}
    ${s.trigger === 'cold-materialise' ? `<div class="field"><label for="sc-mat">Materialised artifact</label>${sel('sc-mat', s.materialise, Object.keys(s.pins).map(a => [a, a]))}</div>` : ''}
    <div class="field"><label for="sc-ver">Running launcher version</label><input type="text" id="sc-ver" value="${esc(s.running.version)}"></div>
    <div class="field"><label><input type="checkbox" id="sc-shim" ${s.running.shimReadable ? 'checked' : ''}> Running shim can be hashed</label></div>
    <div class="field"><label for="sc-lock">Root lock</label>${sel('sc-lock', s.rootLock, [['free', 'free'], ['held', 'held by another prune'], ['error', 'lock call errors (ENOLCK)'], ['foreign', 'a symlink, or not yours']])}</div>
    <div class="field"><label for="sc-claims">claims/</label>${sel('sc-claims', s.claimsDir, [['readable', 'readable'], ['absent', 'absent'], ['unreadable', 'unreadable (mode 0000)'], ['untrusted', 'untrusted (symlink or foreign)'], ['unsearchable', 'listable, unsearchable (mode 0600)']])}</div>
    <div class="field"><label for="sc-trees">trees/</label>${sel('sc-trees', s.treesDir, [['ok', 'present'], ['absent', 'absent'], ['untrusted', 'untrusted'], ['unlistable', 'cannot be listed']])}</div>
    <div class="field"><label for="sc-tclaims">trees/claims/</label>${sel('sc-tclaims', s.treeClaimsDir, [['ok', 'readable'], ['absent', 'absent'], ['unreadable', 'unreadable (mode 0000)'], ['no-search', 'listable, unsearchable (mode 0600)'], ['untrusted', 'untrusted']])}</div>
    <div class="field"><span class="flabel">Clock</span><div class="row"><button class="btn" type="button" data-adv="${H}">+1 h</button><button class="btn" type="button" data-adv="${DAY}">+1 d</button><button class="btn" type="button" data-adv="${7 * DAY}">+7 d</button><button class="btn" type="button" data-adv="${14 * DAY}">+14 d</button></div><span class="small muted">Ages every name and claim.</span></div>
    <div class="field"><label for="sc-session">Start a session of</label><div class="row">${sel('sc-session', s.running.version, versions.map(v => [v, v]))}<button class="btn" type="button" id="sc-start">Claim</button></div><span class="small muted">Runs that version's start-up claim refresh.</span></div>
    <details><summary>Add a name</summary><div class="stack" style="margin-top:8px">
      <div class="field"><label for="sa-kind">Kind</label>${sel('sa-kind', 'sub', [['sub', 'sub-binary + signature'], ['launcher', 'launcher entry + signature'], ['shim', 'shim copy'], ['staging', 'store staging'], ['probe', 'probe'], ['lockdir', 'bootstrap lock directory'], ['foreign', 'any name (test the grammar)'], ['claim', 'version claim'], ['tree', 'tree (pointer, generation, sidecars)'], ['tclaim', 'tree claim'], ['tlock', 'tree lock'], ['temp', 'temp generation']])}</div>
      <div class="field"><label for="sa-name">Version, artifact or name</label><input type="text" id="sa-name" value="1.24.0"></div>
      <div class="field"><label for="sa-dig">Digest label</label><input type="text" id="sa-dig" value="dX"></div>
      <div class="field"><label for="sa-age">Age</label><input type="text" id="sa-age" value="20d"></div>
      <button class="btn" type="button" id="sa-add">Add</button><span class="small muted" id="sa-msg"></span></div></details>`;
}
function recompute() {
  last = run(state);
  const T = $s('#sim-listing');
  if (!T) return;
  const rs = rows(last.w);
  const groups = {};
  rs.forEach(r => (groups[r.group] = groups[r.group] || []).push(r));
  let html = '<thead><tr><th>Name</th><th>Kind</th><th>Age</th><th>Fate</th><th>Rule</th></tr></thead><tbody>';
  for (const [g, list] of Object.entries(groups)) {
    html += `<tr><td colspan="5" class="dir">${esc(g)}</td></tr>`;
    list.forEach(r => {
      const dd = last.dec[r.key] || { fate: 'kept', rule: '' };
      const created = last.created.includes(r.key);
      html += `<tr class="${dd.fate === 'removed' ? 'gone' : ''}" data-row="${esc(r.key)}"><td class="nm"><button type="button" class="loc" style="color:inherit" data-edit="${esc(r.key)}" aria-expanded="${openRow === r.key}">${pretty(r.name)}</button>${created ? ' <span class="chip plain c-info">new</span>' : ''}</td><td class="why">${esc(r.kind)}</td><td class="tnum">${esc(fmtAge(r.age))}</td>
        <td><span class="chip c-${dd.fate === 'removed' ? 'bad' : dd.fate === 'failed' ? 'warn' : 'ok'}">${dd.fate}</span></td><td class="why">${esc(dd.rule)}${dd.ref ? ` <button class="loc" data-open="${esc(dd.ref.path)}" data-line="${dd.ref.line}">${esc(dd.ref.path.split('/').slice(-2).join('/'))}:${dd.ref.line}</button>` : ''}${last.notes[r.key] ? `<div class="small" style="color:var(--warn);margin-top:2px">${esc(last.notes[r.key])}</div>` : ''}</td></tr>`;
      if (openRow === r.key) html += `<tr><td colspan="5" style="background:var(--panel-2)">${editor(r, dd)}</td></tr>`;
    });
  }
  T.innerHTML = html + '</tbody>';
  const rep = last.report;
  const lines = [];
  if (state.trigger === 'verb') {
    lines.push(`$ accelerator cache prune${state.olderThan !== '' && state.olderThan != null ? ' --older-than ' + state.olderThan : ''}`);
    if (last.refused) lines.push(last.refused);
    else { lines.push(last.stdout); [...rep.locks, ...rep.claims, ...rep.untrusted, ...rep.removals].forEach(l => lines.push(l)); if (last.exit) lines.push('the prune did not finish; see the failures above'); }
    lines.push(`exit status ${last.exit}`);
  } else if (state.trigger === 'start-up') lines.push('start-up only: the command runs; nothing is pruned');
  else {
    const warns = [...rep.locks, ...rep.claims, ...rep.untrusted, ...rep.removals];
    last.events.filter(e => /^warn/.test(e.text)).forEach(e => warns.push(e.text.replace(/^warn \(once per process\): /, '')));
    warns.forEach(l => lines.push('WARN ' + l));
    lines.push(`${warns.length ? '' : '(no warnings) '}the dispatch proceeds to exec; reclaimed ${last.reclaimed}`);
  }
  $s('#sim-report').textContent = lines.join('\n');
  const removed = Object.values(last.dec).filter(x => x.fate === 'removed').length;
  $s('#sim-summary').textContent = `${removed} of ${rs.length} names removed · reclaimed count ${last.reclaimed}`;
  $s('#sim-events').innerHTML = last.events.map(e => `<li>${md(e.text, true)}${e.ref ? ` <button class="loc" data-open="${esc(e.ref.path)}" data-line="${e.ref.line}">${esc(e.ref.path.split('/').pop())}:${e.ref.line}</button>` : ''}</li>`).join('');
  const p = PRESETS.find(x => x.id === presetId);
  const mEl = $s('#sim-match');
  if (mEl) mEl.innerHTML = p && p.pristine ? (last.reclaimed === p.expectCount ? `<span class="chip c-ok">model agrees</span> reclaimed count ${last.reclaimed}, as expected` : `<span class="chip c-warn">differs</span> reclaimed ${last.reclaimed}, expected ${p.expectCount}`) : '<span class="muted">Inputs changed from the scenario.</span>';
  ruler();
}
function editor(r, dd) {
  const it = r.item;
  const f = [];
  f.push(`<label class="small">Age <input type="text" data-ed="age" value="${esc(it.age === undefined ? fmtAge(r.age) : fmtAge(r.list === 'trees' ? (r.key.endsWith(':side') ? it.sideAge : it.genAge) : it.age))}" size="7"></label>`);
  if (r.list === 'entries' || r.list === 'claims' || r.list === 'treeClaims') {
    f.push(`<label class="small">Type ${sel('ed-type', it.type, [['file', 'file'], ['dir', 'directory'], ['symlink', 'symlink']]).replace('id="ed-type"', 'data-ed="type"')}</label>`);
    f.push(`<label class="small">Owner ${sel('ed-owner', it.owner, [['self', 'you'], ['other', 'another user']]).replace('id="ed-owner"', 'data-ed="owner"')}</label>`);
  }
  if (r.list === 'entries') {
    const c = classifyRoot(it.name, new Set(state.entries.map(x => expand(x.name))));
    if (c.kind === 'BootstrapLock') { f.push(`<label class="small">pid ${sel('ed-pid', it.pid || 'missing', [['missing', 'missing'], ['blank', 'blank'], ['live', 'a live process'], ['dead', 'a dead process'], ['garbage', 'not a positive integer']]).replace('id="ed-pid"', 'data-ed="pid"')}</label>`); f.push(`<label class="small"><input type="checkbox" data-ed="extra" ${it.extra ? 'checked' : ''}> holds more than pid</label>`); }
    if (c.kind === 'ShimStaging' || c.kind === 'LauncherStaging') f.push(`<label class="small"><input type="checkbox" data-ed="alive" data-pid="${c.pid}" ${(state.livePids || []).includes(c.pid) ? 'checked' : ''}> pid ${c.pid} alive</label>`);
  }
  if (r.list === 'trees') {
    f.push(`<label class="small">Pointer ${sel('ed-ptr', it.pointer, [['current', 'names its generation'], ['none', 'none'], ['../victim', '../victim'], ['/abs', 'an absolute path'], ['other-artifact', "another artifact's generation"], ['other-layout', 'another layout'], ['symlink', 'a symlink'], ['oversize', 'over 256 bytes']]).replace('id="ed-ptr"', 'data-ed="pointer"')}</label>`);
    f.push(`<label class="small">Lease ${sel('ed-lease', it.lease, [['Free', 'free'], ['Held', 'held by another process'], ['Unknown', 'unknown (NFS)']]).replace('id="ed-lease"', 'data-ed="lease"')}</label>`);
    f.push(`<label class="small"><input type="checkbox" data-ed="heldByUs" ${it.heldByUs ? 'checked' : ''}> leased by this dispatch</label>`);
    f.push(`<label class="small"><input type="checkbox" data-ed="race" ${it.race ? 'checked' : ''}> a dispatch leases it mid-removal</label>`);
  }
  if (r.list === 'locks') { f.push(`<label class="small"><input type="checkbox" data-ed="held" ${it.held ? 'checked' : ''}> held by another process</label>`); f.push(`<label class="small"><input type="checkbox" data-ed="errors" ${it.errors ? 'checked' : ''}> lock call errors</label>`); }
  f.push(`<button class="btn" type="button" data-ed-del>Delete</button>`);
  return `<div class="stack" style="gap:8px" data-editor="${esc(r.key)}"><div class="row">${f.join('')}</div>${(dd.trace || []).length ? `<ol class="small" style="margin:0;padding-left:1.3em">${dd.trace.map(t => `<li>${esc(t)}</li>`).join('')}<li><b>${esc(dd.fate)}</b>: ${esc(dd.rule)}</li></ol>` : ''}</div>`;
}
function itemFor(key) {
  const [k, id, part] = key.split(':');
  const list = { e: 'entries', c: 'claims', t: 'trees', tc: 'treeClaims', l: 'locks', x: 'temps', f: 'foreign' }[k];
  return { list, item: state[list].find(i => String(i.id) === id), part };
}
function ruler() {
  const el = $s('#sim-ruler');
  if (!el) return;
  const win = state.trigger === 'verb' && state.olderThan !== '' && state.olderThan != null ? Number(state.olderThan) : CLAIM_WINDOW;
  const lo = Math.log(MIN), hi = Math.log(60 * DAY);
  const x = a => `${(100 * (Math.log(Math.max(MIN, Math.min(60 * DAY, a))) - lo) / (hi - lo)).toFixed(2)}%`;
  let html = '<div class="axis"></div>';
  [[MIN, '1 min'], [H, '1 h backstop'], [REFRESH, '1.4 d refresh'], [CLAIM_WINDOW, '14 d window'], [60 * DAY, '60 d']].forEach(([a, l]) => { html += `<div class="tick" style="left:${x(a)}"></div><div class="tlabel" style="left:${x(a)}">${l}</div>`; });
  html += `<div class="tick" style="left:${x(Math.max(MIN, win))};width:2px;background:var(--bad)"></div>`;
  const src = last ? last.w : state;
  const pins = new Map();
  src.claims.forEach(c => { if (c.age == null) return; const p = parseClaim(c.name); const k = x(clampAge(c.age) || MIN); const e = pins.get(k) || { labels: [], fresh: clampAge(c.age) < win, round: true }; e.labels.push(p ? p.version : c.name); pins.set(k, e); });
  src.treeClaims.forEach(c => { if (c.age == null) return; const k = x(clampAge(c.age) || MIN); const e = pins.get(k) || { labels: [], fresh: clampAge(c.age) < win, round: false }; e.labels.push('‹' + c.digest + '›'); pins.set(k, e); });
  let i = 0;
  for (const [k, e] of pins) { const left = k.trim(); html += `<div class="pin" title="${esc(e.labels.join(', '))}" style="left:${left};background:var(--${e.fresh ? 'ok' : 'bad'});${e.round ? 'border-radius:50%;' : ''}width:10px;height:10px"></div><div class="mark" style="left:${left};top:${i++ % 2 ? 0 : 14}px;color:var(--${e.fresh ? 'ok' : 'bad'})">${esc(e.labels.join(', '))}</div>`; }
  el.innerHTML = html;
}
const $s = s => document.querySelector(s);
function bind() {
  const el = $s('#sim-controls');
  el.innerHTML = controls();
  const on = (id, ev, fn) => { const n = document.getElementById(id); if (n) n.addEventListener(ev, fn); };
  const changed = (rebuild) => { const p = PRESETS.find(x => x.id === presetId); if (p) p.pristine = false; if (rebuild) bind(); else recompute(); };
  on('sc-root', 'change', e => { state.root = e.target.value; changed(true); });
  on('sc-safe', 'change', e => { state.safe = e.target.checked; changed(true); });
  on('sc-unsafe', 'change', e => { state.unsafeReason = e.target.value; changed(); });
  on('sc-trigger', 'change', e => { state.trigger = e.target.value; changed(true); });
  on('sc-older', 'change', e => { state.olderThan = e.target.value === '' ? '' : Number(e.target.value); changed(); });
  on('sc-mat', 'change', e => { state.materialise = e.target.value; changed(); });
  on('sc-ver', 'change', e => { state.running.version = e.target.value.trim() || '1.26.0'; changed(true); });
  on('sc-shim', 'change', e => { state.running.shimReadable = e.target.checked; changed(); });
  on('sc-lock', 'change', e => { state.rootLock = e.target.value; changed(); });
  on('sc-claims', 'change', e => { state.claimsDir = e.target.value; changed(); });
  on('sc-trees', 'change', e => { state.treesDir = e.target.value; changed(); });
  on('sc-tclaims', 'change', e => { state.treeClaimsDir = e.target.value; changed(); });
  $$s('[data-adv]').forEach(b => b.addEventListener('click', () => { const dt = +b.dataset.adv; const bump = o => { if (o.age != null) o.age += dt; }; ['entries', 'claims', 'treeClaims', 'locks', 'temps', 'foreign'].forEach(l => state[l].forEach(bump)); state.trees.forEach(t => { if (t.genAge != null) t.genAge += dt; if (t.sideAge != null) t.sideAge += dt; }); changed(); }));
  on('sc-start', 'click', () => {
    const v = $s('#sc-session').value;
    const sim = JSON.parse(JSON.stringify(state));
    sim.running = { ...sim.running, version: v, id: v === state.running.version ? state.running.id : idFor(v) };
    sim.trigger = 'start-up';
    const res = run(sim);
    state.claims = res.w.claims; state.claimsDir = res.w.claimsDir; state._seq = res.w._seq;
    changed(true);
  });
  on('sa-add', 'click', addName);
  recompute();
}
const $$s = s => Array.from(document.querySelectorAll(s));
function idFor(v) { return tok('id' + v).slice(0, 16); }
function addName() {
  const k = $s('#sa-kind').value, n = $s('#sa-name').value.trim(), dg = ($s('#sa-dig').value.trim() || 'dX').replace(/\W/g, ''), age = parseAge($s('#sa-age').value);
  if (age === undefined) { $s('#sa-msg').textContent = 'Age reads like 20d, 2h, 30min, ? (unknown) or future.'; return; }
  const id = (state._seq = (state._seq || 1000) + 1);
  const push = (list, o) => state[list].push({ id: (state._seq = state._seq + 1), ...o });
  switch (k) {
    case 'sub': push('entries', { name: `vcs-${n}-{${dg}}`, age, type: 'file', owner: 'self' }); push('entries', { name: `vcs-${n}-{${dg}}.minisig`, age, type: 'file', owner: 'self' }); break;
    case 'launcher': push('entries', { name: `accelerator-launcher-${n}-darwin-arm64`, age, type: 'file', owner: 'self' }); push('entries', { name: `accelerator-launcher-${n}-darwin-arm64.minisig`, age, type: 'file', owner: 'self' }); break;
    case 'shim': push('entries', { name: `accelerator-verify-darwin-arm64-{${dg}}`, age, type: 'file', owner: 'self' }); break;
    case 'staging': push('entries', { name: `.tmp-vcs-${n}-{${dg}}-${4000 + (id % 900)}-0`, age, type: 'file', owner: 'self' }); break;
    case 'probe': push('entries', { name: `.accelerator-probe-${4000 + (id % 900)}`, age, type: 'file', owner: 'self' }); break;
    case 'lockdir': push('entries', { name: '.accelerator-lock-linux-x64', age, type: 'dir', owner: 'self', pid: 'dead' }); break;
    case 'foreign': push('entries', { name: n || 'notes.txt', age, type: 'file', owner: 'self' }); break;
    case 'claim': push('claims', { name: `${n}.${idFor(n)}`, age, type: 'file', owner: 'self' }); break;
    case 'tree': state.treesDir = state.treesDir === 'absent' ? 'ok' : state.treesDir; push('trees', { artifact: n || 'browser', platform: 'darwin-arm64', digest: dg, pointer: 'current', generation: true, genAge: age, sidecars: true, sideAge: age, lease: 'Free', heldByUs: false, race: false, layout: 1 }); break;
    case 'tclaim': push('treeClaims', { digest: dg, lid: idFor(dg), age, type: 'file', owner: 'self' }); break;
    case 'tlock': state.treesDir = state.treesDir === 'absent' ? 'ok' : state.treesDir; push('locks', { artifact: n || 'browser', platform: 'darwin-arm64', age, held: false, errors: false }); break;
    case 'temp': state.treesDir = state.treesDir === 'absent' ? 'ok' : state.treesDir; push('temps', { kind: 'temp-gen', artifact: n || 'legacy', platform: 'darwin-arm64', digest: dg, age }); break;
  }
  const p = PRESETS.find(x => x.id === presetId); if (p) p.pristine = false;
  $s('#sa-msg').textContent = 'Added.';
  bind();
}
document.addEventListener('click', e => {
  if (!document.getElementById('sim-listing')) return;
  const pb = e.target.closest('[data-preset]');
  if (pb) { presetId = pb.dataset.preset; const p = PRESETS.find(x => x.id === presetId); state = p.build(); p.pristine = true; openRow = null; ExplorePR.api.rerender(); return; }
  const eb = e.target.closest('[data-edit]');
  if (eb) { openRow = openRow === eb.dataset.edit ? null : eb.dataset.edit; recompute(); return; }
  if (e.target.closest('[data-ed-del]')) { const { list, item } = itemFor(openRow); state[list] = state[list].filter(i => i !== item); openRow = null; const p = PRESETS.find(x => x.id === presetId); if (p) p.pristine = false; bind(); return; }
  if (e.target.id === 'sim-commit') { if (!last) return; state = commit(state, run(state)); delete state._created; openRow = null; const p = PRESETS.find(x => x.id === presetId); if (p) p.pristine = false; bind(); return; }
  if (e.target.id === 'sim-reset') { const p = PRESETS.find(x => x.id === presetId); state = p.build(); p.pristine = true; openRow = null; bind(); }
});
document.addEventListener('change', e => {
  const ed = e.target.closest && e.target.closest('[data-editor]');
  if (!ed) return;
  const { list, item, part } = itemFor(ed.dataset.editor);
  if (!item) return;
  const f = e.target.dataset.ed;
  if (f === 'age') { const a = parseAge(e.target.value); if (a === undefined) return; if (list === 'trees') { if (part === 'side') item.sideAge = a; else item.genAge = a; } else item.age = a; }
  else if (f === 'alive') { const pid = +e.target.dataset.pid; state.livePids = (state.livePids || []).filter(p => p !== pid); if (e.target.checked) state.livePids.push(pid); }
  else if (e.target.type === 'checkbox') item[f] = e.target.checked;
  else item[f] = e.target.value;
  const p = PRESETS.find(x => x.id === presetId); if (p) p.pristine = false;
  recompute();
});
PRESETS[0].pristine = true;
ExplorePR.registerView({ id: 'sim', title: 'Prune simulator', group: 'Read', render: () => vSim(), mount: () => bind() });
})();
