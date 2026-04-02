const API_KEY = process.env.OPENAI_API_KEY;
const DEFAULT_MODEL = process.env.AI_MODEL || "gpt-4o-mini";

export async function complete(
  messages,
  { json: jsonMode = false, temperature = 0.7, model } = {},
) {
  const body = { model: model || DEFAULT_MODEL, messages, temperature };
  if (jsonMode) body.response_format = { type: "json_object" };

  const res = await fetch("https://api.openai.com/v1/chat/completions", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${API_KEY}`,
    },
    body: JSON.stringify(body),
  });

  if (!res.ok) {
    const text = await res.text();
    throw new Error(`AI error ${res.status}: ${text.slice(0, 200)}`);
  }

  const data = await res.json();
  return data.choices?.[0]?.message?.content || "";
}
