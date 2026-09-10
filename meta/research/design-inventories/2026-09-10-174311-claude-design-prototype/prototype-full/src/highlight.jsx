// Lightweight regex-based syntax highlighter for the markdown renderer.
// Supports a handful of languages relevant to the accelerator repo:
// rust, ts/js, python, bash, yaml, toml, json, sql, diff, html.
//
// Each language is an ordered list of [tokenClass, regex] rules. At each
// position the first rule whose regex matches at that position wins; any
// run that no rule matches is emitted as a plain text span.
//
// Capture groups inside patterns MUST be non-capturing (`(?:…)`) — the
// engine counts wrapper groups to identify which rule fired.

const LANG_RULES = {
  rust: [
    ["com",   /\/\/[^\n]*|\/\*[\s\S]*?\*\//],
    ["attr",  /#!?\[[^\]]*\]/],
    ["str",   /b?"(?:\\.|[^"\\])*"|b?'(?:\\.|[^'\\])'/],
    ["lifet", /'[a-z_][a-z0-9_]*\b/],
    ["num",   /\b\d[\d_]*(?:\.\d[\d_]*)?(?:[eE][+\-]?\d+)?(?:[iuf](?:8|16|32|64|128|size)?)?\b/],
    ["kw",    /\b(?:fn|let|mut|pub|use|mod|struct|enum|impl|trait|for|in|if|else|match|return|self|Self|async|await|move|where|as|crate|super|const|static|ref|loop|while|break|continue|dyn|box)\b/],
    ["lit",   /\b(?:true|false|None|Some|Ok|Err)\b/],
    ["typ",   /\b(?:bool|i8|i16|i32|i64|i128|u8|u16|u32|u64|u128|usize|isize|f32|f64|String|Vec|Option|Result|Box|Arc|Rc|HashMap|BTreeMap|str|char|Path|PathBuf)\b|\b[A-Z][A-Za-z0-9_]*\b/],
    ["macro", /\b[a-z_][a-z0-9_]*!(?=[\(\[{])/],
    ["fn",    /\b[a-z_][a-z0-9_]*(?=\s*\()/],
    ["pun",   /->|=>|::|&mut\b|[{}()\[\];,.<>:&|*]/],
  ],
  typescript: [
    ["com",   /\/\/[^\n]*|\/\*[\s\S]*?\*\//],
    ["str",   /"(?:\\.|[^"\\\n])*"|'(?:\\.|[^'\\\n])*'|`(?:\\.|[^`\\])*`/],
    ["kw",    /\b(?:const|let|var|function|return|if|else|for|while|do|break|continue|switch|case|default|import|export|from|as|class|extends|implements|interface|type|enum|new|this|super|null|undefined|async|await|throw|try|catch|finally|in|of|typeof|instanceof|void|public|private|protected|readonly|static|abstract|namespace|declare)\b/],
    ["lit",   /\b(?:true|false|null|undefined)\b/],
    ["typ",   /\b(?:string|number|boolean|any|unknown|never|object|Promise|Array|Map|Set|Record|Partial|Readonly|ReadonlyArray)\b|\b[A-Z][A-Za-z0-9_]*\b/],
    ["num",   /\b\d[\d_]*(?:\.\d[\d_]*)?(?:[eE][+\-]?\d+)?n?\b|\b0x[0-9a-fA-F_]+\b/],
    ["fn",    /\b[a-zA-Z_$][a-zA-Z0-9_$]*(?=\s*\()/],
    ["pun",   /=>|\.{3}|[{}()\[\];,.?:!&|<>+\-*\/=%]/],
  ],
  python: [
    ["com",   /#[^\n]*/],
    ["str",   /(?:[rbuRBU]{0,2})(?:"""[\s\S]*?"""|'''[\s\S]*?'''|"(?:\\.|[^"\\\n])*"|'(?:\\.|[^'\\\n])*')/],
    ["deco",  /@[A-Za-z_][A-Za-z0-9_.]*/],
    ["kw",    /\b(?:def|class|return|if|elif|else|for|while|in|not|and|or|is|import|from|as|with|try|except|finally|raise|pass|break|continue|lambda|yield|async|await|global|nonlocal|assert|del)\b/],
    ["lit",   /\b(?:None|True|False)\b/],
    ["bn",    /\b(?:self|cls|print|len|range|str|int|float|bool|list|dict|set|tuple|type|isinstance|enumerate|zip|map|filter|open|sum|min|max|abs|sorted|reversed|any|all)\b/],
    ["num",   /\b\d[\d_]*(?:\.\d[\d_]*)?(?:[eE][+\-]?\d+)?\b/],
    ["fn",    /\b[a-zA-Z_][a-zA-Z0-9_]*(?=\s*\()/],
    ["pun",   /[{}()\[\]:;,.<>+\-*\/=%!&|]/],
  ],
  bash: [
    ["com",   /#[^\n]*/],
    ["str",   /"(?:\\.|[^"\\\n])*"|'[^'\n]*'/],
    ["var",   /\$\{[^}]+\}|\$[A-Za-z_][A-Za-z0-9_]*|\$\d/],
    ["heredoc", /<<-?\s*['"]?[A-Z_]+['"]?/],
    ["kw",    /\b(?:if|then|else|elif|fi|for|in|do|done|while|until|case|esac|function|return|export|local|readonly|source|set|unset|trap|shift|alias|declare|eval)\b/],
    ["bn",    /\b(?:echo|cd|pwd|ls|cat|grep|sed|awk|cut|sort|uniq|head|tail|find|xargs|curl|wget|jq|yq|git|make|cargo|npm|pnpm|yarn|mise|pytest|mypy|pylint|python|python3|node|ruby|rg|sha256sum|tar|gzip|chmod|chown|mkdir|touch|cp|mv|rm|ln|test|true|false)\b/],
    ["flag",  /(?:^|(?<=\s))--?[A-Za-z][A-Za-z0-9\-_]*/],
    ["num",   /\b\d+\b/],
    ["pun",   /[{}()\[\];,|&<>=]/],
  ],
  yaml: [
    ["com",   /#[^\n]*/],
    ["key",   /(?:^|(?<=[\s\-]))[A-Za-z_][A-Za-z0-9_\-.]*(?=\s*:)/m],
    ["str",   /"(?:\\.|[^"\\\n])*"|'[^'\n]*'/],
    ["lit",   /\b(?:true|false|null|yes|no|on|off|~)\b/i],
    ["num",   /\b\d+(?:\.\d+)?\b/],
    ["anchor",/[&*][A-Za-z_][A-Za-z0-9_\-]*/],
    ["pun",   /[\-:?|>{}\[\],]/],
  ],
  toml: [
    ["com",   /#[^\n]*/],
    ["header",/^\s*\[\[?[^\]]+\]\]?/m],
    ["key",   /(?:^|(?<=[\s,{]))[A-Za-z_][A-Za-z0-9_\-.]*(?=\s*=)/m],
    ["str",   /"""[\s\S]*?"""|'''[\s\S]*?'''|"(?:\\.|[^"\\\n])*"|'[^'\n]*'/],
    ["lit",   /\b(?:true|false)\b/],
    ["num",   /\b\d+(?:\.\d+)?(?:[eE][+\-]?\d+)?\b/],
    ["pun",   /[=\[\]{},.]/],
  ],
  json: [
    ["com",   /\/\/[^\n]*|\/\*[\s\S]*?\*\//], // jsonc tolerance
    ["key",   /"(?:\\.|[^"\\\n])*"(?=\s*:)/],
    ["str",   /"(?:\\.|[^"\\\n])*"/],
    ["lit",   /\b(?:true|false|null)\b/],
    ["num",   /-?\b\d+(?:\.\d+)?(?:[eE][+\-]?\d+)?\b/],
    ["pun",   /[{}\[\]:,]/],
  ],
  sql: [
    ["com",   /--[^\n]*|\/\*[\s\S]*?\*\//],
    ["str",   /'(?:''|[^'\n])*'/],
    ["kw",    /\b(?:SELECT|FROM|WHERE|JOIN|LEFT|RIGHT|INNER|OUTER|CROSS|ON|GROUP|BY|ORDER|HAVING|LIMIT|OFFSET|INSERT|INTO|VALUES|UPDATE|SET|DELETE|CREATE|TABLE|VIEW|INDEX|ALTER|DROP|AS|AND|OR|NOT|NULL|IS|IN|EXISTS|CASE|WHEN|THEN|ELSE|END|UNION|ALL|DISTINCT|WITH|RETURNING|USING|PRIMARY|FOREIGN|KEY|REFERENCES|DEFAULT|CONSTRAINT|UNIQUE|CHECK|CASCADE)\b/i],
    ["typ",   /\b(?:INTEGER|INT|BIGINT|SMALLINT|TEXT|VARCHAR|CHAR|BOOLEAN|BOOL|TIMESTAMP|TIMESTAMPTZ|DATE|TIME|JSONB?|UUID|SERIAL|BIGSERIAL|REAL|DOUBLE|FLOAT|DECIMAL|NUMERIC|BYTEA)\b/i],
    ["fn",    /\b[A-Za-z_][A-Za-z0-9_]*(?=\s*\()/],
    ["num",   /\b\d+(?:\.\d+)?\b/],
    ["pun",   /[(),;.*=<>+\-]/],
  ],
  html: [
    ["com",   /<!--[\s\S]*?-->/],
    ["doctype",/<!DOCTYPE[^>]+>/i],
    ["tag",   /<\/?[a-zA-Z][a-zA-Z0-9\-]*/],
    ["attr",  /\b[a-zA-Z][a-zA-Z0-9\-]*(?==)/],
    ["str",   /"(?:\\.|[^"\\\n])*"|'(?:\\.|[^'\\\n])*'/],
    ["pun",   /\/?>|=/],
  ],
  diff: [
    ["dhdr",  /^(?:diff --git|index|\+\+\+|---).*$/m],
    ["dhunk", /^@@[^\n]*@@.*$/m],
    ["dadd",  /^\+[^\n]*$/m],
    ["ddel",  /^-[^\n]*$/m],
  ],
  css: [
    ["com",   /\/\*[\s\S]*?\*\//],
    ["str",   /"(?:\\.|[^"\\\n])*"|'(?:\\.|[^'\\\n])*'/],
    ["atrule",/@[a-zA-Z\-]+/],
    ["sel",   /[.#][a-zA-Z_][a-zA-Z0-9_\-]*|::?[a-zA-Z\-]+|&|\*/],
    ["var",   /--[a-zA-Z_][a-zA-Z0-9_\-]*/],
    ["num",   /\b\d+(?:\.\d+)?(?:px|em|rem|%|vh|vw|s|ms|deg|fr|ch|ex)?\b|#[0-9a-fA-F]{3,8}\b/],
    ["fn",    /\b[a-zA-Z_][a-zA-Z0-9_\-]*(?=\()/],
    ["lit",   /\b(?:none|auto|inherit|initial|unset|currentColor|transparent|true|false|hidden|visible|absolute|relative|fixed|sticky|flex|grid|block|inline|inline-block|inline-flex|center|left|right|top|bottom|bold|normal|italic)\b/],
    ["prop",  /\b[a-z\-]+(?=\s*:)/],
    ["pun",   /[{};:,()]/],
  ],
};

// Aliases.
LANG_RULES.rs   = LANG_RULES.rust;
LANG_RULES.ts   = LANG_RULES.typescript;
LANG_RULES.tsx  = LANG_RULES.typescript;
LANG_RULES.js   = LANG_RULES.typescript;
LANG_RULES.jsx  = LANG_RULES.typescript;
LANG_RULES.py   = LANG_RULES.python;
LANG_RULES.sh   = LANG_RULES.bash;
LANG_RULES.shell = LANG_RULES.bash;
LANG_RULES.zsh  = LANG_RULES.bash;
LANG_RULES.yml  = LANG_RULES.yaml;
LANG_RULES.postgres = LANG_RULES.sql;
LANG_RULES.psql = LANG_RULES.sql;

// Build a combined per-language regex once, lazily.
const PATTERN_CACHE = {};
function patternFor(lang) {
  if (PATTERN_CACHE[lang]) return PATTERN_CACHE[lang];
  const rules = LANG_RULES[lang];
  if (!rules) return null;
  const re = new RegExp(rules.map(([_, r]) => "(" + r.source + ")").join("|"), "gm");
  const types = rules.map(([t]) => t);
  return (PATTERN_CACHE[lang] = { re, types });
}

// Tokenize src into [{type, text}, …].
function tokenize(src, lang) {
  const pat = patternFor((lang || "").toLowerCase());
  if (!pat) return [{ type: "plain", text: src }];
  const out = [];
  let last = 0;
  let m;
  // Reset lastIndex defensively — we mutate a shared regex object.
  pat.re.lastIndex = 0;
  while ((m = pat.re.exec(src))) {
    if (m.index > last) out.push({ type: "plain", text: src.slice(last, m.index) });
    // Find which top-level capture group fired.
    let groupIdx = -1;
    for (let i = 1; i < m.length; i++) {
      if (m[i] !== undefined) { groupIdx = i - 1; break; }
    }
    out.push({ type: pat.types[groupIdx] || "plain", text: m[0] });
    last = pat.re.lastIndex;
    if (m[0].length === 0) pat.re.lastIndex++;
  }
  if (last < src.length) out.push({ type: "plain", text: src.slice(last) });
  return out;
}

// Render a highlighted code block as a React fragment.
function HighlightedCode({ code, lang }) {
  const tokens = tokenize(code, lang);
  return (
    <code>
      {tokens.map((t, i) => t.type === "plain"
        ? <React.Fragment key={i}>{t.text}</React.Fragment>
        : <span key={i} className={`tk tk-${t.type}`}>{t.text}</span>)}
    </code>
  );
}

// Pretty label for the corner badge.
const LANG_LABELS = {
  rust: "Rust", rs: "Rust",
  typescript: "TypeScript", ts: "TypeScript", tsx: "TSX",
  javascript: "JavaScript", js: "JavaScript", jsx: "JSX",
  python: "Python", py: "Python",
  bash: "Bash", sh: "Shell", shell: "Shell", zsh: "Zsh",
  yaml: "YAML", yml: "YAML",
  toml: "TOML",
  json: "JSON", jsonc: "JSONC",
  sql: "SQL", postgres: "PostgreSQL", psql: "PostgreSQL",
  html: "HTML",
  css: "CSS",
  diff: "Diff", patch: "Patch",
  text: "Plain", "": "Plain",
};
function langLabel(lang) {
  const k = (lang || "").toLowerCase();
  return LANG_LABELS[k] || (k ? k.toUpperCase() : "Plain");
}

Object.assign(window, { HighlightedCode, langLabel, tokenize });
