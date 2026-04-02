const ALLOWED_ORIGIN =
  process.env.ALLOWED_ORIGIN || "https://rockywearsahat.github.io";

const corsHeaders = {
  "Access-Control-Allow-Origin": ALLOWED_ORIGIN,
  "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
  "Access-Control-Allow-Headers": "Content-Type, Authorization",
  "Access-Control-Max-Age": "86400",
};

export function options() {
  return new Response(null, { status: 204, headers: corsHeaders });
}

export function json(data, status = 200) {
  return new Response(JSON.stringify(data), {
    status,
    headers: { ...corsHeaders, "Content-Type": "application/json" },
  });
}

export function fail(message, status = 400) {
  return json({ error: message }, status);
}
