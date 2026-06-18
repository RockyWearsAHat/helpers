const ALLOWED_ORIGIN =
  process.env.ALLOWED_ORIGIN || "https://rockywearsahat.github.io";

const corsHeaders = {
  "Access-Control-Allow-Origin": ALLOWED_ORIGIN,
  "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
  "Access-Control-Allow-Headers": "Content-Type, Authorization",
  "Access-Control-Max-Age": "86400",
};

/** Respond to a CORS preflight (OPTIONS) request with a 204 and CORS headers. */
export function options() {
  return new Response(null, { status: 204, headers: corsHeaders });
}

/** JSON response with the given body and status, including CORS headers. */
export function json(data, status = 200) {
  return new Response(JSON.stringify(data), {
    status,
    headers: { ...corsHeaders, "Content-Type": "application/json" },
  });
}

/** JSON error response: `{ error: message }` at the given status (default 400). */
export function fail(message, status = 400) {
  return json({ error: message }, status);
}
