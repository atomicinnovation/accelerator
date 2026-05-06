// Canonical error envelope formatter shared across all executor modules.

const VALID_CATEGORIES = new Set(['usage', 'protocol', 'browser', 'bootstrap', 'filesystem']);
export const PROTOCOL = 1;

export function makeError({ error, message, category, retryable = false, details }) {
  if (!VALID_CATEGORIES.has(category)) {
    throw new Error(`Unknown error category: ${category}`);
  }
  const env = { protocol: PROTOCOL, error, message, category, retryable };
  if (details !== undefined) env.details = details;
  return env;
}

export function protocolMismatch(got) {
  return makeError({
    error: 'protocol-mismatch',
    message: `Protocol mismatch: client sent "protocol": ${got}, executor requires "protocol": ${PROTOCOL}. Update the agent body to send "protocol": ${PROTOCOL}.`,
    category: 'protocol',
    retryable: false,
    details: { expected: PROTOCOL, got },
  });
}

export function emitErrorAndExit(env, code = 1) {
  process.stderr.write(JSON.stringify(env) + '\n');
  process.exit(code);
}
